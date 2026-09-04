# PDF Viewer — Build & Test System Plan

Status: draft, revised 2026-07-26. Scope: infrastructure only.
Project principles live in `/CLAUDE.md` and take precedence over anything here.

## 1. Stack decisions

| Area | Decision | Notes |
|---|---|---|
| Language | Rust | Eliminates the dominant CVE class in PDF viewers |
| Images | `zune-jpeg`, `hayro-jbig2`, `hayro-jpeg2000` | All pure Rust; JBIG2/JPX decode in the sandbox (ADR 0014) |
| Rasterizer | **CPU first, GPU behind a trait** | `tiny-skia` → `vello`/wgpu (ADR 0002) |
| Fonts | `skrifa` | Memory-safe FreeType replacement; Type1 in-tree, Type 3 in `pdf-model` (its glyphs are content streams) |
| Windowing | `winit` | Qt dropped — see below |
| Dialogs | `ashpd` (XDG portal) | Native KDE dialogs without a Qt dependency |
| Accessibility | `AccessKit` | AT-SPI on Linux |
| Parallelism | `rayon` | Tiles, image decode, thumbnails — not the parser |
| Deflate | `flate2` + `zlib-rs` | Pure Rust at ~C speed |
| Spec model | Arlington PDF Model | Generated validation layer, see §5 |
| Sandbox | seccomp-BPF + Landlock | **Built.** Image codecs run in it; `--no-sandbox` opts out |
| Speed baseline | `hayro` (`tools/hayro-compare`) | The only other pure-Rust renderer, so a fair comparison — see §4a |

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
than any cross-viewer comparison, because both consume the same input. Measured at mean
0.0136/255 between `tiny-skia` and Vello — see ADR 0004.

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

**Image codecs.** `zune-jpeg` / `zune-png` cover the common cases in pure Rust.

This paragraph used to say that **JBIG2 and JPEG 2000 have no mature pure-Rust
implementation**, that they are historically severe attack surfaces (FORCEDENTRY was a JBIG2
integer overflow), and that *these two decoders alone justify the sandbox*. The first clause
stopped being true: `hayro-jbig2` and `hayro-jpeg2000` are pure-Rust decoders, both
`#![forbid(unsafe_code)]`, and both are now used, with `default-features = false` so that
their optional SIMD backend — the only `unsafe` either would reach — stays out of the tree.

The sandbox was built anyway, and both codecs run inside it, but the justification changed
and is worth restating honestly: it is panic containment, an enforceable memory ceiling, and
the architecture principle 3 already required — not the containment of C. See ADR 0014,
which records what the dependency costs as well as what it buys.

## 2. Workspace layout

```
pdf-viewer/
├─ crates/
│  ├─ pdf-spec/       # Arlington codegen output + validation  [forbid(unsafe_code)]
│  ├─ pdf-syntax/     # lexer, objects, xref, streams          [forbid(unsafe_code)]
│  ├─ pdf-model/      # document model, page tree              [forbid(unsafe_code)]
│  ├─ pdf-font/       # skrifa integration, §9.6.5.2/§9.6.5.4  [forbid(unsafe_code)]
│  ├─ pdf-render/     # display list, backend trait            [forbid(unsafe_code)]
│  ├─ render-cpu/     # tiny-skia backend — oracle + startup path
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
- [x] Vulkan packages: vulkan-radeon, vulkan-swrast, validation layers, mupdf
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
- **the launch-path gate**: cold open, time-to-first-page, page-turn latency, memory
  high-water, measured with a cold page cache, plus the cold graphics bring-up principle 2 makes
  a gate of its own. `crates/viewer-ui/tests/launch_path.rs` with its bands in
  `doc/checks/launch-path.toml`; `tools/state.sh launch` prints it and `doc/todo/02` §2 runs it.
  **Not `criterion`**, which this line named for nine hundred rounds and which is the wrong shape
  for the question: four of the five figures are about a *process*, and a benchmark harness that
  measures a function in a warm loop cannot see a cold open, a driver's bring-up or a
  high-water mark. ADR 0884 is the construction and ADR 0885 what it found.
  Startup latency is a first-class requirement — see `CLAUDE.md` principle 2 for the
  rules that follow from it. **This line used to end "GPU initialisation stays off the critical
  path and page one renders on the CPU backend while the device is created", and that is the
  opposite of what the owner decided**: page one goes to the graphics device by choice, so
  bring-up is *on* the critical path and is a number to keep small rather than a cost to move
  aside. Corrected in the nine-hundred-and-twenty-second session, against a principle that has
  said so since the two-hundred-and-seventies.
- Miri on the pure-Rust core; ASan/UBSan on any FFI
- `cargo-deny`, `cargo-audit`
- **conformance gate** (§5a) — citations checked against the standard's own clause index,
  quotations verified verbatim, ledger coverage ratcheted. The third gate, and the only one
  whose denominator is the specification rather than a corpus. Not built.

### The viewer

`cargo run --release -p viewer-ui --bin pdf-viewer -- document.pdf` opens a real file.
Arrow keys or Page Up/Down turn pages, `+`/`-`/`0` zoom, a drag selects text, `s` saves what
was changed; the title bar names anything on the page that could not be drawn, because a
viewer that shows an incomplete page confidently is worse than one that admits the gap.

**Since the hundred-and-thirty-second session that binary is a *consumer* rather than the
program.** Everything about documents, pages, clicks, selection and editing is `viewer-core`
— `Command` in, `Event` out, `Query` → `Answer` beside them, with no windowing or graphics
type in its API — and what is left in `viewer-ui` is a window, a keyboard, a GPU and the two
decisions a host owns: which files a document may name, and what to do when one asks for a
password. `doc/ui-boundary.md` is that interface's
specification, and ADRs 0116 to 0121 are
its argument; the second consumer is `viewer-core/tests/headless.rs`, which drives the whole
state machine with no display at all.

### Phase 5 — De-risking spikes (before PDF code)
- **A.** ~~Headless CPU render → byte-deterministic output.~~ **Done.** `render-cpu` on
  `tiny-skia`; fills, strokes and nested clips verified, output byte-identical across
  runs, PNG artefact written for inspection. 9 tests. Confirmed `tiny-skia` covers all
  sixteen PDF blend modes.
- **B.** ~~GPU backend on Vello/wgpu.~~ **Done (headless part).** Offscreen render with
  no window or display server; cross-backend agreement with `render-cpu` verified within
  measured tolerances; row-padding readback covered. 7 tests. See ADR 0004.
  The interactive half is `cargo run --release --example spike-window -p viewer-ui`:
  winit 0.30 window, Vello scene blitted to the swapchain, resize handling, frame times
  on stdout. **Confirmed working by the project owner**; the agent cannot run it, having
  no X authority cookie. It calls `render_gpu::build_scene`, the same translation the
  headless tests exercise, so the window cannot diverge from what CI checks.
- **C.** ~~Arlington TSV → generated validation tables.~~ **Done.** 611 objects, 3973 key
  rows, `static` tables generated in ~0.5 s with zero startup cost. Verified against ISO
  32000-2 tables 29 and 31. 12 tests. See ADR 0003.
- **D.** ~~Sandboxed child process.~~ **Done.** `pdf-sandbox` starts a confined worker on
  first use: resource limits, a Landlock domain permitting nothing, and a seccomp-BPF
  allow-list of 23 system calls with `KillProcess` for everything else. JBIG2 and JPEG 2000
  decode inside it. Confinement is tested by re-executing the test binary as a probe that
  tries to open a file and to bind a socket — both die by `SIGSYS` — and both probes were
  confirmed to *pass* when lockdown is removed. Requests cross pipes rather than shared
  memory: a copy is under a millisecond and shared memory would need `unsafe`, which the
  crate that exists to contain dangerous code should not be the first to spend. 9 tests.
  See ADR 0014.
- **E.** ~~Harness end-to-end.~~ **Done.** `tools/pdfref` with the triangulation rule,
  size normalisation, failure artefacts and a divergence-survey CLI. Our CPU render is
  byte-identical to mupdf on the fixture. 15 tests. See ADR 0005.

## 4. Reference-comparison harness

Three independent reference implementations are installed; their *agreement* is the
evidence we rely on.

| Renderer | Command | Version | Votes? |
|---|---|---|---|
| poppler | `pdftoppm -r 150 -png -aa yes` | 26.07.0 | yes |
| mupdf | `mutool draw -r 150 -o out.png` | 1.28.0 | yes |
| ghostscript | `gs -sDEVICE=png16m -r150` | 10.07.1 | yes |
| hayro | `pdfref-hayro` (ours, wrapping the crate) | 0.7.1 | **no — see below** |
| pdfium | *to add (AUR)* | de-facto standard — Chrome's renderer | would |

### Independence is a property of a renderer, and it is now in the type

The word "independent" above was an assumption for six sessions and then cost something.
`mupdf` and `ghostscript` **both link `jbig2dec`**: on a page whose image is JBIG2 they are
one implementation, and the gate duly reported seven pages as contradicting us where in fact
`jbig2dec` renders a blank page or one strewn with noise. `Reference::independence` now
records this, and `Reference::voting` is what the gate iterates, so a renderer that cannot
supply evidence cannot silently be counted as supplying it.

`hayro` is added on the same principle and never votes. It shares `skrifa`, `flate2`,
`zune-jpeg`, `hayro-jbig2` and `hayro-jpeg2000` with us — not one format's decoder but the
substrate of nearly every page — so there is no useful subset on which its agreement would
be evidence. It is rendered for the artefacts of pages that are *not* agreement, which is
where a fourth reading actually helps, and it is the only renderer this project can compare
its **speed** against without confounding the language.

### Expect inexact agreement

Exact pixel equality is impossible even *between* poppler and mupdf. A harness built on
exact comparison produces false positives until it gets ignored, so tolerance is a design
requirement, not a concession. Measured (ADR 0005):

| Content | References differ from each other by | Usable gate |
|---|---|---|
| Vector (fixture) | mean 0.002–0.047, worst tile 0.4–1.1 | tight: worst tile 5.0 |
| Text (spec PDFs) | worst tile 26–28, 2.7% of pixels | weak only: worst tile 40 |

Page dimensions also disagree: A4 is 595.276 units wide, so poppler and mupdf render 596
pixels where ghostscript renders 595. Reconciled by cropping to the common size when the
spread is at most one pixel per axis, and always reported.

**Pixel comparison cannot police text.** The disagreement on text pages is confined to
glyph outlines and one-pixel borders — hinting and antialiasing, not error — and its
magnitude exceeds any tolerance that would still catch a wrong glyph. Text correctness is
therefore metric 2's job, not metric 3's.

### Metrics ladder (cheapest and strictest first)

1. **Geometry** — page count, dimensions, rotation. Exact match required.
2. **Text** — our extraction vs `pdftotext`. Validates encoding and `ToUnicode`
   *independently of rendering*, isolating a whole error class. **Now load-bearing, not
   optional:** measurement showed pixel comparison cannot police text at all (see below),
   so this is the only metric that can.
3. **Structural similarity** — SSIM / blurred difference, per-corpus tolerance.
4. **Localized max error** — tile the page, report worst tile. Mean metrics average away a
   single missing glyph on a dense page; this is the one people forget.

### Triangulation rule

- ≥2 references agree and we differ → real bug, fail the build.
- All references disagree with each other → ambiguous spec corner; record as
  known-divergent, do not fail.

This is what keeps the suite trustworthy enough to stay enabled.

**Now running over the whole corpus, with the bound taken from the references themselves.**
`crates/pdf-model/tests/oracle.rs` applies the rule to every page of the 974 pdf.js corpus
documents and page one of the 14 specification PDFs — 1794 pages — in about 34 seconds once
the references' renders are remembered (ADR 0020): roughly 97 seconds of processor time is
ours and 42 the three external renderers, where before the cache the latter was some 1020. Consensus is
still decided by the fixed tolerance, but *our* deviation is judged
against twice the disagreement the consensus references show among themselves on that page:
a fixed number cannot serve both a page of flat fills, where they agree to a worst tile of
0.4, and a page of small text, where they differ by 26 among themselves. Only pages we claim
to draw completely are gated, every contradicted page is named in the source, and both a new
disagreement and a stale entry fail the build. See ADR 0011.

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

## 4a. hayro, and what it changes

`hayro` is a nine-crate pure-Rust PDF renderer published to crates.io. This project already
depends on three of its crates for JBIG2, JPEG 2000 and CCITT (ADR 0014), so the relationship
needs stating rather than leaving implicit. `doc/hayro vs this project.md` has the long
version; what belongs in a plan is what it changes. (Two of its figures have since moved:
the oracle contradicts us on **108 of 1478** pages rather than 120 of 1340, and **692 of
974** first pages report nothing rather than 587. Its analysis is unaffected.)

**It is a library and this is an application.** That explains most of the differences and it
sets where the differentiators have to be: startup latency, the sandbox, the GPU path, the
viewer itself, and a correctness standard anchored to the specification rather than to
consensus with other implementations. That is roughly what `CLAUDE.md` already says.

**It is ahead on feature completeness, and not narrowly.** Its regression suite is 1000+
PDFs and it has closed things we have written down as gaps: transparency groups, encryption,
predefined `CMap`s (a whole crate, `hayro-cmap`), Type1 fonts, transparency. Type 3 fonts,
optional content and inline images have since landed here. Our own oracle contradicts us on
108 of 1478 pages we claim to draw.

**Where the direction of inference must not reverse.** `hayro-jbig2` and `hayro-jpeg2000`
are dependencies implementing ITU-T T.88 and T.800, with exactly the status `zune-jpeg` has
for `DCTDecode` and `skrifa` has for fonts. Principle 5 is unchanged by adopting them: if one
disagrees with its standard, the answer is an upstream issue, never a local workaround and
never a revised expectation. Treating "what hayro does" as the definition of done is the
inference direction `CLAUDE.md` forbids, and adopting three of its crates makes that
temptation stronger rather than weaker.

### Items taken from the comparison

1. **`CCITTFaxDecode` through `hayro-ccitt`.** Already in the dependency tree as
   `hayro-jbig2`'s MMR decoder, so the last absent image codec is nearly free: an arm in
   `image.rs` and the `Bilevel` round trip JBIG2 already uses. 5 corpus first pages.
2. **Their crate boundaries are worth copying where ours are missing.** `hayro-cmap` as its
   own crate is a better shape than our "embedded `CMap`s are 14 documents in the text gap":
   a `CMap` parser is self-contained, independently testable and independently fuzzable,
   which is the argument that made `pdf-syntax` separate from `pdf-model` in the first place.
3. **The `simd` feature pattern.** `hayro-jpeg2000` defaults vectorisation on and documents
   that turning it off "eliminates any usage of unsafe in this crate as well as its
   dependencies". The consumer picks the point on the curve and the cost is stated at both
   ends. If anything here is ever vectorised, that is the shape — and it fits the rule that
   an optimisation carries the benchmark that justifies it. `--no-sandbox` is the same idea
   on a different axis.
4. **A published-crate discipline.** `missing_docs` is already enforced; actually publishing
   `pdf-syntax` would force the API to be defensible to someone who is not us.
5. **Corpus scale.** 20 000 images scraped from real PDFs for one codec is an order of
   magnitude past our 974 documents. Trap 8 says a corpus finds what documents contain rather
   than what the specification says; the converse is also true, and 974 is a small sample of
   what producers emit.

### Speed, measured rather than assumed

`tools/hayro-compare` exists for this. Both renderers are Rust, both forbid unsafe, both
rasterise on the CPU single-threaded, so what a timing difference measures is the code rather
than the language — which is not true of any of the other three references.

Over 698 corpus pages we claim to draw completely, best of three passes, alternating:

| | before | after two fixes |
|---|---|---|
| total, ours | 22.8 s | **7.1 s** |
| total, hayro | 38.6 s | 41.8 s |
| **median page** | 1.61× slower | **1.62× slower** |
| worst page | 225× slower | 34× slower |
| pages we are faster on | 117 of 698 | 116 of 698 |

Two statements, both true, and they answer different questions. **In aggregate we are now
5.9× faster**, because `hayro`'s distribution has a long tail and ours no longer does. **On
the median page we are still 1.62× slower**, and that number did not move, because the two
fixes were to outliers. The typical corpus page is small and text-heavy, and nothing has yet
been profiled there — that is the next measurement, not the next optimisation.

The two fixes were found with `callgrind` and are written up in "Where the time went" in
`doc/HANDOVER.md`: a per-pixel unpacking loop that cost more than the JPEG codec it was
unpacking, and a mesh-subdivision criterion missing a size term. Both follow the standing
rule — an optimisation must be justified by a benchmark and explained by a comment — and the
second improved fidelity as well as speed, which the oracle measured.

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

Resolved in Spike C: `SinceVersion`'s predicates are a closed set of two shapes and are
modelled exactly; `Required` is uniformly `fn:IsRequired(...)`. `SpecialCase` and
predicate-bearing `PossibleValues` are carried verbatim and unevaluated, because an
evaluator needs a document to evaluate against and so belongs after `pdf-syntax`. See
ADR 0003 for the measured breakdown.

## 5a. Conformance ledger and citation checking

The sibling of §5, and the half Arlington cannot supply. **The Arlington model is the object
model, not the semantics**: it says `/BaseEncoding` must be one of three names and nothing
about what those encodings contain. Nothing in the tree tracks which of the standard's
*requirements* are implemented, so the only answers to "how much of PDF do we support" are a
corpus count and a prose self-assessment — one measures demand, the other has been wrong
twice. `CLAUDE.md` principle 5 states what conformance means; this section is the machinery
that makes the claim checkable.

**The ledger.** `doc/conformance/ledger.toml`, one row per numbered subclause of clauses
7–14 — 823 of them, 663 leaves — generated once with every row `unreviewed`, and changed
only by someone who has read that clause against this code.

**And, since the three-hundred-and-sixtieth session, one row per number of the eight normative
annexes**: 52 more, taking the ledger to 875. The population was the eight numbered clauses
because that is how the standard's *body* is organised, and nothing had noticed that D, E, F, I,
K, L, O and Q say **normative** on their own title lines. `ClauseNumber` could not parse `K.2`,
so a citation to one was malformed, a quotation from one was uncheckable and a row for one was
unwritable — the instrument's silence was total rather than partial. ADR 0206.

| Status | Means |
|---|---|
| `implemented` | Every normative requirement in the clause is executed. Names the code site and the test. |
| `partial` | Names which requirements are implemented, which are not, and what is *reported* for the remainder. |
| `reported` | Deliberately not implemented *yet*; detected and reported at runtime rather than skipped silently. Still owed. |
| `silent` | Not implemented, and **nothing says so**: a document exercising the clause is drawn wrong without a word. |
| `inapplicable` | The requirement cannot reach this program: it describes a press rather than a screen (§10.6's halftones, on the standard's own condition — ADR 0204), or it is a permission this program declines and has no code to point at (§14.11.2.2's page-boundary guidelines). **Two situations under one word**, which ADR 0205 had to separate by hand; every such note says which it means. **Not** the same as excluded, and not the same as a permission *exercised*, which is `implemented` where there is code to name — §10.7.2's flatness is the standing example. |
| `writer-side` | The requirement addresses a PDF *generator*: what a file shall contain, laid out how. Principle 5 also lists this as an exclusion, but it gets its own status because it is a property of the clause rather than a choice about scope. **The exclusion is authoring, not writing** — §7.5.6's incremental update of what a person did is in scope and implemented (ADR 0121) — so a row is `writer-side` only where the requirement falls on whoever *creates* the structure. The seven rows were re-read against that in session 137; six stayed and §7.2.2 moved to `implemented`, because a tree that writes has to write ASCII tokens. |
| `out-of-scope` | **Only** for a clause covered by principle 5's closed exclusion list, and the row must name which entry covers it. |
| `unreviewed` | Nobody has read this clause against this code. The initial state of all 823, and of the 52 annex rows added in the three-hundred-and-sixtieth session. |

**`silent` was added while filling the first rows, and it is the status worth hunting.** Every
missing *subsystem* in this tree reports — `LZWDecode`, encryption, Type 3 fonts — because
whoever decided not to build it wrote the report the same afternoon. The gaps that ship are the
ones *inside* something implemented, where the operator is handled and the code path exists:
`Tr` parsed with four of its eight modes changing a clip nobody built (fixed in the
thirteenth session, ADR 0022), `/SMask` honoured while `/Mask` beside it was not, knockout
groups compositing as though they were not knockouts.
Reading the clause is the only thing that finds those; this is where the finding goes. The
ninth session's first pass produced two, §11.4.6 and §8.11.4.4, and both were invisible to
every other instrument here. The eleventh found a third silence and *removed* one. The one it
found does not get a `silent` row, and that is worth recording: §8.9.5.2's defaults are
implemented and its general `/Decode` array is not, so the row is `partial` and the silence
lives in its note. A one-word status cannot say "half of this is quiet", so a reader hunting
silence by status alone will miss it. The one it removed was made to report: an `/SMask` whose sample grid is not its image's
(§11.6.5.2 Table 143) was silently not applied, and `issue16263.pdf` drew black bars across a
page of text because of it.

**`out-of-scope` is the status that would rot first, so it is the one the checker
constrains.** `CLAUDE.md` principle 5 fixes a closed list of exclusions — clause 13, XFA,
script-driven form behaviour, writer-side requirements — and a row may carry `out-of-scope`
only with an `exclusion` field naming one of them. The valid values are a closed enum in the
checker, so widening the list means editing principle 5 and the checker together, in a commit
that says so. Without that constraint the status becomes the graveyard every clause goes to
once it turns out to be difficult, which is precisely the escape hatch principle 5 refuses.
A clause that is merely unimplemented is `unreviewed`, `partial` or `reported` — never
`out-of-scope`.

The rest of the vocabulary exists to keep five different situations from wearing one word:
the project *choosing* (`out-of-scope`), the project *not knowing* (`unreviewed`), the
project *owing out loud* (`reported`, and `partial` for part of a clause), the project *owing
in silence* (`silent`), and the requirement having no meaning for a screen (`inapplicable`).
`out-of-scope` and `inapplicable` are permanent; **`writer-side` is not, and session 137 is the proof** — a clause that addressed only a generator became this tree's the moment it grew one. The rest are four different
kinds of debt, and the ledger's headline number is how much of each is left. The distinction
between the last two kinds is the one this project cares about most: a gap that reports is a
gap you can schedule, and a gap that does not is a gap that ships.

Clause 13's subclauses are generated into the ledger like every other, and marked
`out-of-scope` with their exclusion named, rather than omitted. An exclusion that is invisible
is indistinguishable from an oversight.

TOML rather than a Markdown table because the checker parses it and a prose table drifts the
moment someone reflows it; any human-readable summary is generated *from* it, the same
relationship `pdf-spec` has with the Arlington TSVs.

**The checker.** `tools/conformance`: a library, a `ledger` binary that generates and
regenerates the rows, and `tests/conformance.rs`, the gate. Its only dependency is
`thiserror`, which every crate here uses; the ledger's format is read by `toml_subset.rs`
rather than by a TOML crate, because the conformance gate is the last thing that should stop
running because a dependency did not. That module accepts a documented subset and **rejects**
the rest by line — valid TOML outside the subset fails to read rather than being misread,
which is the property that makes a restricted reader safe to build. It reads
`doc/md/ISO_32000-2_sponsored_EC3.md`, which since session 311 is **not tracked in the clear** —
ISO's text is free to obtain and not free to redistribute, so it lives inside
`doc/specifications.zip` and a developer unpacks it (ADR 0187). It has no skip path all the same,
and unlike the pdf.js submodule that is deliberate: a missing corpus costs a ratchet, and a
missing standard costs every citation in the tree its only check. It:

- builds the clause index from the file's 860 `##` headings, each giving a clause number, a
  title and a line range;
- fails on a `§` citation in Rust source naming a clause the standard does not have;
- fails on a rustdoc blockquote whose text does not occur within its cited clause's range,
  compared with whitespace collapsed and `![Image]` lines skipped;
- fails on a ledger row whose clause does not exist, or which claims `implemented` without
  naming a code site and a test that exist;
- fails on an `out-of-scope` row whose `exclusion` is not one of principle 5's closed
  entries — the constraint that keeps the status from becoming a graveyard;
- prints the coverage summary and **ratchets it**: `unreviewed` may only fall, and a clause
  cited by code may never be `unreviewed`.

Three caveats. `doc/md/` is a *conversion*: a quotation it cannot find may be a conversion
artefact rather than a bad quote, so check `doc/`'s PDF before editing the comment — and one
heading number (`14.8.4.7.3`) occurs twice, in the body and in the corrigendum that renumbers
it, so both spans are searched. Second, the checker verifies that a citation is *well-formed
and honest*, never that the code implements the clause; only a person reading the clause can
set a ledger status, which is the point of having statuses rather than a computed percentage.
Third, **it does not scan its own crate**, and `conformance::NOT_SCANNED` says why at length:
its comments name `§8.9.6.5` and `§11.4.5.6` deliberately, because those are the two wrong
numbers it was built to catch.

Two ratchets, both in the gate and both two-directional. `UNREVIEWED_CEILING` may only fall.
`REVIEW_OWED` names the clauses the code cites whose rows are still `unreviewed` — 33 of them,
found by the rule's first run — and a clause not on the list fails immediately, while a clause
on it that *has* been reviewed must be deleted from it. It is a list rather than a count
because filling 36 rows in one sitting to make a gate pass is exactly the rubber stamp the
ledger exists to prevent.

On quoting the standard: `doc/md/` is already committed, so a short attributed quotation in a
source file is no new exposure inside this repository — but quotes in source travel with any
code that is later published or excerpted, which the markdown does not. Keep them to the
load-bearing sentence, which is also the right length for readability.

**How it gets filled.** By clause family, from ordinary work — all four subclauses of §8.9.6
while implementing image `/Mask`, all of §8.11 while implementing optional content. A family
is the right unit because that is how the standard distributes its requirements, and because
§9.6.5.4 was missed for the opposite reason: nobody had read §9.6.5 as a unit. A one-pass
review of all 823 is the kind of task that is abandoned partway and afterwards remembered as
complete.

**Status: built in the ninth session.** ADR 0016 has the whole argument, including what was
decided against. Where it stands on its first green run:

| | |
|---|---|
| citations checked, all naming clauses the standard has | 317 |
| rustdoc blockquotes verified verbatim | 6 |
| ledger rows | 823 |
| reviewed | 135, of which 81 are clause 13's exclusion |
| `unreviewed`, and the number that may only fall | 688 |
| cited clauses still owing a review (`REVIEW_OWED`) | 25 |

Six sessions later, at the end of the fifteenth: 479 citations, 25 quotations, 33 distinct
tables, 171 rows reviewed, **652** `unreviewed`, and 23 clauses still owing a review. At the end
of the twenty-first: 891 citations, 68 quotations, 51 distinct tables, 262 rows reviewed,
**561** `unreviewed`, and 16 clauses still owing a review. At the end of the twenty-fourth: 1210
citations, 111 quotations, 72 distinct tables, 348 rows reviewed, **475** `unreviewed`, and the
same 16 owing a review.

The measurements that justified it were 146 citations over 36 distinct clause numbers, two of
which named clauses that do not exist, and three of five sampled quotations that were
paraphrases inside quotation marks. All five are now fixed, and each of the four checks was
confirmed to fail when its defect is put back.

The first clause-family reviews paid for themselves before the ledger had a hundred rows: the
`/Mask` citations were *still* wrong after being corrected once (§8.9.6.2 is stencil masking;
`/Mask` naming another image is §8.9.6.3), stencil masking turned out to be implemented with
nothing pinning it, and §8.9.6.2's interpolation sentence is not implemented at all.

The tenth session's reviews kept the rate up, and the clause that produced most was the one
that looked like a formality. §8.6.8 is a table of twelve operators everybody knows are
implemented; reading it gave three findings — that its restriction on colour operators governs
uncoloured *tiling patterns* as well as `d1` glyph descriptions, that its list is not what
Table 111's parenthesis implies, and that `cs`/`CS` must set an initial colour which is black
in only three of its six cases. None of the three was what the session was looking for, and
none of them reported anything at runtime.

The eleventh session read §8.9 as a whole family — twelve rows — and found four more of the
same kind, plus one the checker is *structurally* unable to find: `/SMaskInData` cited
§8.9.5.4, which is a real clause about alternate images and not the one that defines the
entry. A wrong number the standard happens to have passes every automated check there is, and
only reading the clause catches it. Reading §8.9.5.4 properly then produced the one case where
`/Alternates` decides what is on the page — a base image hidden by `/OC` should be replaced by
its first visible alternate — which was silent and now reports.

The eighteenth session read §11.7 — fourteen rows — beside building §11.5's soft masks, and
the pairing is why: §11.6.5.1's `/BC` is stated in a group's *blending colour space*, and
§11.7.2 is the clause that says what such a space is. It produced one row satisfied by a
decision taken for another reason (§11.7.3: a spot colour is converted through its tint
transform everywhere here, which is exactly what the clause requires inside a soft mask), two
`inapplicable` rows whose subject is a marking device this tree does not have (§11.7.5.1 and
§11.7.5.2), and **a family of six `silent` rows: §11.7.4, overprinting**. `/OP`, `/op` and
`/OPM` are read nowhere and 63 of the corpus's first-page `/ExtGState` dictionaries set one of
the two booleans, so a document that enables overprinting is composited through Normal with
nothing said. Six rows for one gap is the same recording §11.4.6, §11.6.6 and §11.3.7.3 got
for transparency groups: a reader of any of them should find it.

The fifteenth session read §11.3.7, §11.5 and the whole of §11.6 — seventeen rows, §11.6.4
having been the fourteenth's — and produced three defects and two `silent` rows. The defects:
a shading dropped §11.6.4.4's alpha constant, because a shading replaces the colour rather
than tinting it and the constant went with the colour; a `/BM` array took the first *name*
rather than the first mode this reader recognises (§11.6.3); and §11.6.2's rule that the
portions of one object are not composited with one another was neither implemented nor
reported for a filled-and-stroked path. The first of those had made `alphatrans.pdf`
contradicted by all three references for four sessions, filed under a group that named its
fonts. The `silent` rows are §11.6.6 and §11.3.7.3, which are the transparency-group gap
§11.4.6 already owns, recorded where a reader of those clauses would look for it.

The thirteenth session read §9.3 and §9.4 as two families — thirteen rows — and produced a
defect, a `silent` row and a limit of the checker itself. The defect is §9.3.3: word spacing
is a rule about a code's *encoded length* and was implemented as a rule about its value, so an
`Identity-H` string containing the bytes `00 20` was pushed right by `Tw` for every one of
them. The `silent` row is §9.3.8, text knockout, whose `/TK` entry nothing looks for. And the
limit is the one below.

**Table numbers are now checked, weakly and honestly.** The tree cited "§9.3.6 Table 106" for
the text rendering modes in four comments, two tests and a written report; the modes are Table
104, and Table 106 is the text-*positioning* operators. Every automated check passed, because
the clause exists and the table exists and only the pair is wrong. The obvious gate — the
clause beside a table reference must be one the standard discusses that table in — was built
and then **rejected by its own output**: it fails fourteen of this tree's twenty-five
references and all fourteen are correct writing, because a comment about one clause routinely
names a table belonging to another. A gate that is more exceptions than rule is not a gate. So
the assertion is the weaker true one, that the number names a table the standard has, and the
gate *prints the title of every distinct table the tree cites* — thirty-one lines, in which
"Table 106 — Text-positioning operators" beside a file about rendering modes is visible at a
glance. A checker cannot read a comment's intent; a person reading thirty-one lines can.

## 6. Security architecture

Memory safety is necessary, not sufficient.

- **Built, for the image codecs.** `pdf-sandbox` confines a worker process with resource
  limits, Landlock and seccomp-BPF, and JBIG2 and JPEG 2000 decode in it. The worker receives
  one image's bytes over a pipe and returns samples over another; it never learns which
  document they came from. `--no-sandbox` decodes in process instead, which is a supported
  choice for trusted documents and prints what it gives up. The *rest* of the renderer still
  runs in the main process — that is the remaining half of this item.
- GPU-touching code ideally its own process — drivers are unsafe C and exploitable.
- Explicit memory/time budgets against decompression bombs and pathological content.
- Any C image codec confined to the sandboxed process. There is currently no C in the tree
  at all; JBIG2 and JPX are pure Rust (ADR 0014).
- AcroForm JavaScript, if ever supported, is a separate sandboxing problem. Defer, but
  don't design it out.

Crates: `landlock`, `seccompiler`, `rustix`, and `libc` for system-call numbers.
`pdf-sandbox` is `#![forbid(unsafe_code)]` — all four expose safe interfaces.

## 7. Environment

Verified: rustc/cargo 1.97.1, cmake 4.4.0, ninja 1.13.2, clang 22.1.8, poppler 26.07.0,
mupdf-tools 1.28.0, ghostscript 10.07.1, qpdf 12.3.2, imagemagick 7.1.2.27, python 3.14.6
+ pillow 12.3.0 + numpy 2.5.1, xdg-desktop-portal 1.22.1 + `-kde` 6.7.3 + `-gtk` 1.15.3,
kio/kconfig/ki18n 6.28.0.

GPU: AMD Strix (Radeon 880M / 890M), RDNA 3.5. Session: X11 (`DISPLAY=:0`).

### Dependency policy notes

- `winit` is built with `default-features = false` and without `wayland-csd-adwaita`.
  That feature draws Adwaita client-side decorations via
  `ab_glyph -> owned_ttf_parser -> ttf-parser`, which is unmaintained
  (RUSTSEC-2026-0192) with no safe upgrade. KWin provides server-side decorations, so
  the loss only affects compositors that offer none — chiefly GNOME.
- Internal crates carry an explicit `version` alongside `path`. A bare path dependency
  is a wildcard requirement, which the ban policy rejects and which would block
  publishing.
- `CC0-1.0` is allowed: a public-domain dedication, reached via `hexf-parse` under
  naga.
- `cargo-deny` is installed in the agent user's `~/.cargo/bin`, so the supply-chain
  gate can be reproduced locally before pushing.

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
