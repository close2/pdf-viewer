# PDF Viewer — Build & Test System Plan

Status: **standing** — what the build, test and measurement system *is*. Scope: infrastructure.
Read by: a round asking how this tree is built, tested, measured or held to the specification.
Project principles live in `/CLAUDE.md` and take precedence over anything here; a session's story
is its ADR's and `doc/history/`'s, and `CLAUDE.md`'s comment rule binds every sentence below —
the current reason, the ADR by number, and a retired sentence deleted rather than annotated.

## 1. Stack decisions

**The table is [`doc/stack.md`](stack.md)** — one row per choice, what is deliberately not in it,
and what each dependency costs. This section is the *rationale*, which is what that file points
here for; `doc/crate-map.md` says which crate each choice lives in. One table in one file, because
two documents stating one choice is how they drift.

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

**Why there is a CPU backend at all, and why it was built first.** It reached a correct rendered
page soonest, and what it buys permanently is a **same-scene oracle**: diffing our own CPU and GPU
backends on an identical display list is far tighter than any cross-viewer comparison, because both
consume the same input — measured at mean 0.0136/255 between `tiny-skia` and Vello (ADR 0004). It
is also what draws a frame the graphics device refuses. What it is *not* is the startup path: page
one goes to the device by the owner's decision (`CLAUDE.md` principle 2).

**Why the flagship window is not a toolkit's.** `viewer-ui` is `winit` and this tree's own
drawing, so the launch path crosses no FFI boundary and the whole stack reads in one language,
which is principle 4 in the build rather than in a comment. Accessibility does not depend on a
widget set either: `viewer-accessibility` puts §14.7's tree on AT-SPI through `AccessKit` for a
custom-drawn interface. What it costs is a mature widget set and a toolkit's free i18n, menus and
shortcuts. ADR 0001.

**Qt is in the tree, as a host rather than as the flagship's toolkit.** `crates/viewer-qt` is a
Qt 6 Widgets application on `viewer-core`'s boundary, one of the two places C++ is written here (ADR 0246)
and `kio/`'s worker the other;
`crates/viewer-gtk` is the GTK4 one (ADR 0244). A host is a consumer of `Command`/`Event`, so the
toolkits cost the core nothing and cost the flagship's build nothing: `doc/ui-boundary.md` is the
interface and `doc/crate-map.md` says which crate is which.

**No crate depends on `ashpd`.** A document arrives on the command line or by Ctrl + O, which in
`quorra-gtk` and `quorra-qt` is the toolkit's own file dialogue and in `quorra` a line over the page
a path is typed into, every route ending in `viewer_host::open_chosen` (ADR 1275). The decision
§12.7.6.4 actually puts on a host — which files a document may name — is `viewer-host`'s policy
rather than a dialog, and `cargo metadata` is the authority for what is linked.

**Image codecs.** `zune-jpeg` and `zune-png` cover the common cases. `hayro-jbig2` and
`hayro-jpeg2000` are pure-Rust JBIG2 and JPEG 2000 decoders, both `#![forbid(unsafe_code)]`, taken
with `default-features = false` so that their optional SIMD backend — the only `unsafe` either
would reach — stays out of the tree. **No C or C++ library is linked into anything a document's
bytes reach**, which is the claim that matters here: the C and C++ this tree does contain is the two
faces it offers *outward*, `viewer-ffi`'s and `pdf-vfs-ffi`'s headers and example callers,
`viewer-qt`'s `cxx` bridge and `kio/`'s worker. The one C library a host links for a job of its
own is `ring`, the provider under `rustls` in `viewer_host::submit`: it carries a form a person let
leave and the server's answer, and parses nothing a document wrote. Principle 3's letter still asks
for it to be confined, so it is argued in ADR 1291 section 4 and put to the owner in
`doc/questions/Q130`.

**What the sandbox is for.** Both codecs run inside it, and the reason is panic containment, an
enforceable memory ceiling and the architecture principle 3 requires — not the containment of C,
which there is none of. They are the codecs that run confined because JBIG2 and JPEG 2000 are
historically the severest attack surfaces in a PDF reader: FORCEDENTRY was a JBIG2 integer
overflow. ADR 0014 records what the dependency costs as well as what it buys.

## 2. Workspace layout

One cargo workspace, whose members are `crates/*`, `tools/*` and `raster/crates/*`. **`cargo
metadata --no-deps` is the authority for what that is** and [`doc/crate-map.md`](crate-map.md) is
one row per crate saying what each is responsible for; neither a count nor a tree drawing lives
here, because a hand-maintained list of the members drifts from the manifest that defines them.
`kio/` is deliberately *outside* the workspace — a C++ `MODULE` plugin with no `Cargo.toml`, named
by no manifest, so a machine with no KDE builds and tests the whole workspace unchanged (ADR 0869).

`forbid(unsafe_code)` is load-bearing: it makes "untrusted bytes never reach unsafe code"
compiler-enforced rather than conventional. Every crate that a document's bytes reach forbids it.
The crates that only `deny` it are the three a foreign ABI passes through — `viewer-qt`'s `cxx`
bridge, `viewer-ffi` and `pdf-vfs-ffi` — and `viewer-qt`'s
`only_the_three_named_crates_in_the_tree_lift_the_denial` is the list, asserted rather than
written down, so a fourth crate lifting the denial fails a test instead of editing a document.
`render-gpu` is permitted `unsafe` by its own header — a surface from a raw window handle
eventually needs it — and contains none, the offscreen path needing none.

## 3. Foundation, the build, and the test layers

**Foundation.** A git repository, with `rustup` and a `rust-toolchain.toml` pinning an exact
stable version, nightly beside it for Miri, and `rustfmt.toml`, `clippy.toml`, `deny.toml` and
`.gitignore` in the root. CI is GitHub Actions in `.github/workflows/ci.yml`: fmt, clippy, the
tests with `mesa-vulkan-drivers` for a software Vulkan adapter, `cargo-deny`, and an advisory Miri
job.

**The crate graph.** Acyclic and layered, with the safety attributes of §2 in place: `pdf-render`
defines the display list, the `Rasterizer` trait and `TargetSpec`, and the three rasterisers depend
on it and on nothing else of ours. Clean under `clippy::pedantic` with warnings as errors, and
`cargo fmt --check` clean. **Every lint exception is an `#[expect(..., reason = "...")]` rather
than a bare `allow`**, so an exception that stops being necessary becomes a warning instead of
lingering invisibly — trap 7, and the one trap that binds any round writing Rust at all.

**The build system is Cargo and nothing else.** No CMake, no moc, no Corrosion; `kio/`'s CMake is
outside the workspace and reached by no build script. `build.rs` in `pdf-spec` runs the Arlington
codegen (§5), and `crates/pdf-sandbox/build.rs` bakes the confined worker's path.

### The test layers

- `cargo-nextest` — unit tests
- `proptest` — parser round-trips
- `cargo-fuzz` — from the first parser commit; every crasher becomes a regression test. The
  targets live in `fuzz/`, which is **its own workspace**: `--all` and `--workspace` do not reach
  them (trap 23).
- the reference-comparison harness (§4), and the self-golden beside it
- **the launch-path gate**: cold open, time-to-first-page, page-turn latency, memory high-water,
  measured with a cold page cache, plus the cold graphics bring-up principle 2 makes a gate of its
  own. `crates/viewer-ui/tests/launch_path.rs` with its bands in `doc/checks/launch-path.toml`;
  `tools/state.sh launch` prints it and `doc/todo/02` §2 runs it. **Not `criterion`**, which is the
  wrong shape for the question: four of the five figures are about a *process*, and a benchmark
  harness that measures a function in a warm loop cannot see a cold open, a driver's bring-up or a
  high-water mark. ADR 0884 is the construction and ADR 0885 what it found. Page one goes to the
  graphics device by the owner's decision, so bring-up is *on* the critical path and is a number to
  keep small rather than a cost to move aside — `CLAUDE.md` principle 2 has the rules that follow.
- Miri on the pure-Rust core; ASan/UBSan on any FFI
- `cargo-deny`, `cargo-audit`
- **the conformance gate** (§5a) — citations checked against the standard's own clause index,
  quotations verified verbatim, ledger coverage ratcheted. The third gate, and the only one whose
  denominator is the specification rather than a corpus. `cargo test -p conformance` runs it.

### The viewer

`cargo run --release -p viewer-ui --bin quorra -- document.pdf` opens a real file. Arrow keys or
Page Up/Down turn pages, `+`/`-`/`0` zoom, a drag selects text, `s` saves what was changed; the
title bar names anything on the page that could not be drawn, because a viewer that shows an
incomplete page confidently is worse than one that admits the gap.

**That binary is a *consumer* rather than the program.** Everything about documents, pages,
clicks, selection and editing is `viewer-core` — `Command` in, `Event` out, `Query` → `Answer`
beside them, with no windowing or graphics type in its API — and what is left in `viewer-ui` is a
window, a keyboard, a graphics device and the decisions a host owns: which files a document may
name, what to do when one asks for a password, and whether a form it composes may leave the
machine (ADR 1291). [`doc/ui-boundary.md`](ui-boundary.md) is that
interface's specification and ADRs 0116 to 0121 are its argument; `viewer-core/tests/headless.rs`
drives the whole state machine with no display at all, and
[`doc/state-of-play.md`](state-of-play.md) lists every consumer on the boundary.

### What the de-risking spikes settled, and still hold

Five questions were answered before any PDF code was written, and each answer is still a property
something in the tree asserts. ADRs 0003, 0004, 0005 and 0014 are the arguments; `doc/history/`
has the rounds.

- **A CPU render is byte-deterministic.** `render-cpu`: fills, strokes and nested clips, output
  byte-identical across runs, and all sixteen of §11.3.5's blend modes covered. The coverage under
  that is now this tree's own exact area rather than `tiny-skia`'s lattice, which is what makes the
  backend an oracle and not a second opinion (ADR 1082).
- **The GPU backend renders headless**, with no window and no display server, and agrees with
  `render-cpu` within measured tolerances — including the row-padding readback. The interactive
  half is `cargo run --release --example spike-window -p viewer-ui`, which calls the same
  `render_gpu::build_scene` the headless tests exercise, so the window cannot diverge from what CI
  checks. The agent has no X authority cookie and cannot run it; `doc/environment.md` says what it
  can do instead.
- **The Arlington TSVs generate `static` validation tables** with zero startup cost, verified
  against ISO 32000-2 Tables 29 and 31. §5 is the design.
- **A confined worker is confined**, and the kernel says so rather than the source: `pdf-sandbox`
  starts one on first use under resource limits, a Landlock domain permitting nothing and a
  seccomp-BPF allow-list with `KillProcess` for everything else, and probes that try to open a
  file and to bind a socket die by `SIGSYS` — each probe confirmed to *pass* when lockdown is
  removed, which is trap 13's rule applied to a sandbox. Requests cross pipes rather than shared
  memory: a copy is under a millisecond, and shared memory would need `unsafe`, which the crate
  that exists to contain dangerous code should not be the first to spend.
- **The harness works end to end.** `tools/pdfref` carries the triangulation rule, size
  normalisation, failure artefacts and a divergence-survey CLI.

## 4. Reference-comparison harness

Three independent reference implementations are installed; their *agreement* is the
evidence we rely on.

| Renderer | Command | Votes? |
|---|---|---|
| poppler | `pdftoppm -r 150 -png -aa yes` | yes |
| mupdf | `mutool draw -r 150 -o out.png` | yes |
| ghostscript | `gs -sDEVICE=png16m -r150` | yes |
| hayro | `pdfref-hayro` (ours, wrapping the crate) | **no — see below** |
| pdfium | not installed | would — the de-facto standard, Chrome's renderer |

Each renderer's version is the machine's, and `pdftoppm -v`, `mutool -v` and `gs --version` print
it; a version written down here would be a number nothing checks.

### Independence is a property of a renderer, and it is now in the type

The word "independent" above was an assumption, and it cost something.
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

**It runs over the whole corpus, with the bound taken from the references themselves.**
`crates/pdf-model/tests/oracle.rs` applies the rule to every page of the pdf.js corpus documents
and page one of the specification PDFs, at a cost the references' remembered renders decide rather
than their re-invocation (ADR 0020); `tools/state.sh oracle` prints where it stands and what it
covers. Consensus is decided by the fixed tolerance, but *our* deviation is judged against twice
the disagreement the consensus references show among themselves **on that page**: a fixed number
cannot serve both a page of flat fills, where they agree to a worst tile of 0.4, and a page of
small text, where they differ by 26 among themselves. Only pages we claim to draw completely are
gated, every contradicted page is named in the source, and both a new disagreement and a stale
entry fail the build. See ADR 0011.

### Goldens

Snapshots of *our own* output, separate from reference comparison, catching commit-to-commit
regressions including in deliberately-divergent areas. `crates/pdf-model/tests/raster_golden.rs`
is the gate: it walks the tracked first pages through `render-cpu` and holds three digests and an
outcome word per page **by name** in `raster_golden.tsv`, so a page that moves fails naming the
page and the layer — interpreter, rasteriser or report — that moved it (ADR 1016). The oracle sees
disagreement with other renderers; this is what sees *change* in our own, which matters most on
the pages the oracle can only call ambiguous.

**Goldens are shared across adapters rather than per-backend.** For the vector path RADV and
lavapipe are byte-identical, because Vello's compute pipeline has no driver-dependent
fixed-function rasterisation, and a test pins that property so its loss is noticed. Checked on one
vendor and simple scenes only — text and images may still diverge. See ADR 0004.

On failure, emit side-by-side plus difference heatmap as CI artifacts — diagnosis must take
seconds.

### Corpus

`doc/*.pdf` is a strong start: ISO 32000-2 is large, complex, tagged, font-heavy, real.
Add pdf.js and pdfium corpora, veraPDF, Isartor (malformed files), Arlington's own
`TestGrammar/test/` fixtures, and the growing fuzz corpus. Large corpora fetched on
demand, not committed.

Where all open-source renderers are jointly wrong, Acrobat is the gold standard and is not
scriptable on Linux — keep a small manually-captured Acrobat golden set.

## 4a. hayro, and what it changes

`hayro` is a nine-crate pure-Rust PDF renderer published to crates.io. This project depends on
three of its crates for JBIG2, JPEG 2000 and CCITT (ADR 0014), so the relationship is stated rather
than left implicit. `doc/hayro vs this project.md` has the long version; what belongs here is what
it changes. Where a sentence below needs a figure, `tools/state.sh` prints the current one.

**It is a library and this is an application.** That explains most of the differences and it
sets where the differentiators have to be: startup latency, the sandbox, the GPU path, the
viewer itself, and a correctness standard anchored to the specification rather than to
consensus with other implementations. That is roughly what `CLAUDE.md` already says.

**It is ahead on feature completeness, and not narrowly.** Its regression suite is 1000+ PDFs and
it has closed things this tree wrote down as gaps: transparency groups, encryption, predefined
`CMap`s (a whole crate, `hayro-cmap`), Type1 fonts. Type 3 fonts, optional content and inline
images are in this tree. How many pages our own oracle contradicts us on is `tools/state.sh
oracle`'s to say.

**Where the direction of inference must not reverse.** `hayro-jbig2` and `hayro-jpeg2000`
are dependencies implementing ITU-T T.88 and T.800, with exactly the status `zune-jpeg` has
for `DCTDecode` and `skrifa` has for fonts. Principle 5 is unchanged by adopting them: if one
disagrees with its standard, the answer is an upstream issue, never a local workaround and
never a revised expectation. Treating "what hayro does" as the definition of done is the
inference direction `CLAUDE.md` forbids, and adopting three of its crates makes that
temptation stronger rather than weaker.

### Items taken from the comparison

1. **`CCITTFaxDecode` through `hayro-ccitt`**, taken: it was already in the dependency tree as
   `hayro-jbig2`'s MMR decoder, and it decodes in the confined worker beside JBIG2 and JPEG 2000,
   `crates/pdf-sandbox/src/decode.rs` being the only place in the tree any of the three codestreams
   is looked at.
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
5. **Corpus scale.** 20 000 images scraped from real PDFs for one codec is an order of magnitude
   past the corpus this tree gates on. Trap 8 says a corpus finds what documents contain rather
   than what the specification says; the converse is also true, and any corpus here is a small
   sample of what producers emit. `tools/state.sh corpus` says which populations are on this disk.

### Speed, measured rather than assumed

`tools/hayro-compare` exists for this. Both renderers are Rust, both forbid unsafe, both rasterise
on the CPU single-threaded, so what a timing difference measures is the code rather than the
language — which is not true of any of the other three references.

**[`doc/performance.md`](performance.md) §4 owns the measurement**, one column per time it was
taken, and the rule that comes with it: quote a total against a total taken the same afternoon,
because two independent programs do not slow down together and the ratio is the measurement while
the totals are the machine. The aggregate and the median answer different questions — their
distribution has a long tail and ours does not, and on the median page we are still the slower —
so only quoting both is honest.

## 5. Arlington PDF Model

Cloned at `doc/arlington-pdf-model`, with `tsv/2.0/` defining the PDF 2.0 object model;
`ls doc/arlington-pdf-model/tsv/2.0 | wc -l` says how many TSVs that is. Columns: `Key, Type, SinceVersion, DeprecatedIn, Required,
IndirectReference, Inheritable, DefaultValue, PossibleValues, SpecialCase, Link, Note`.

**Plan: generate the validation layer, don't hand-write it.** A `build.rs` step in
`pdf-spec` turns the TSVs into typed accessors and validation tables. Benefits:

- Spec conformance becomes reviewable *data* rather than thousands of hand-written checks.
- Version-awareness (`SinceVersion` / `DeprecatedIn`) comes free.
- `Link` encodes the object graph, giving typed traversal.
- Directly serves principles 1 and 4: no shortcuts, and legible to a reader.

Resolved when the codegen was built (ADR 0003): `SinceVersion`'s predicates are a closed set of
two shapes and are modelled exactly; `Required` is uniformly `fn:IsRequired(...)`. `SpecialCase` and
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

**The ledger.** `doc/conformance/ledger.toml`, one row per numbered subclause of the population
below, generated once with every row `unreviewed`, and changed only by someone who has read that
clause against this code. `tools/state.sh ledger` prints how many rows there are and how they
stand.

**The population is clause 6, clauses 7 to 14, and the eight normative annexes** — D, E, F, I, K,
L, O and Q, each of which says *normative* on its own title line. Two of those three groups were
added after the first generation, and each time the lesson was the same: an instrument that walks
the ledger's own covered numbers cannot report a clause the ledger does not cover, so its silence
was total rather than partial (ADRs 0206, 0984).

**The population is therefore a checked claim rather than a written one.** `check` counts `shall`
under every clause and annex of the standard and reports one covered by neither the population nor
an argued exclusion, so the next clause left out is a build failure instead of a silence.

| Status | Means |
|---|---|
| `implemented` | Every normative requirement in the clause is executed. Names the code site and the test. |
| `partial` | Names which requirements are implemented, which are not, and what is *reported* for the remainder. |
| `departed` | Every requirement of the clause is executed except the one the note names, which was decided against with its cost recorded. Nothing is owed. The note's first sentence says what was departed from and names the ADR that decided it and priced it; `tools/state.sh` counts it as its own figure, never folded into `implemented` or `partial`. The owner's word, added in answer to `doc/questions/Q63` (ADR 1119). |
| `reported` | Deliberately not implemented *yet*; detected and reported at runtime rather than skipped silently. Still owed. |
| `silent` | Not implemented, and **nothing says so**: a document exercising the clause is drawn wrong without a word. |
| `inapplicable` | The requirement cannot reach this program: it describes a press rather than a screen (§10.6's halftones, on the standard's own condition — ADR 0204), or it is a permission this program declines and has no code to point at (§14.11.2.2's page-boundary guidelines). **Two situations under one word**, which ADR 0205 had to separate by hand; every such note says which it means. **Not** the same as excluded, and not the same as a permission *exercised*, which is `implemented` where there is code to name — §10.7.2's flatness is the standing example. |
| `writer-side` | The requirement addresses a PDF *generator*: what a file shall contain, laid out how. Principle 5 also lists this as an exclusion, but it gets its own status because it is a property of the clause rather than a choice about scope. **The exclusion is authoring, not writing** — §7.5.6's incremental update of what a person did is in scope and implemented (ADR 0121), and RFC 0002 §10's serializer emits structure and never content — so a row is `writer-side` only where the requirement falls on whoever *creates* the structure. §7.2.2 is `implemented` rather than `writer-side`, because a tree that writes has to write ASCII tokens. |
| `out-of-scope` | **Only** for a clause covered by principle 5's closed exclusion list, and the row must name which entry covers it. |
| `unreviewed` | Nobody has read this clause against this code. The initial state of every row, whenever the population grows. |

**`silent` is the status worth hunting.** Every missing *subsystem* in this tree reports —
`LZWDecode`, encryption, Type 3 fonts — because whoever decided not to build it wrote the report
the same afternoon. The gaps that ship are the ones *inside* something implemented, where the
operator is handled and the code path exists: `Tr` parsed with four of its eight modes changing a
clip nobody built (ADR 0022), `/SMask` honoured while `/Mask` beside it was not, knockout groups
compositing as though they were not knockouts. Reading the clause is the only thing that finds
those, and every such finding was invisible to every other instrument here.

**A one-word status cannot say "half of this is quiet".** §8.9.5.2's defaults are implemented and
its general `/Decode` array is not, so that row is `partial` and the silence lives in its note — a
reader hunting silence by status alone will miss it.

**`out-of-scope` is the status that would rot first, so it is the one the checker
constrains.** `CLAUDE.md` principle 5 fixes a closed list of exclusions — clause 13, XFA,
script-driven form behaviour, writer-side requirements — and a row may carry `out-of-scope`
only with an `exclusion` field naming one of them. The valid values are a closed enum in the
checker, so widening the list means editing principle 5 and the checker together, in a commit
that says so. Without that constraint the status becomes the graveyard every clause goes to
once it turns out to be difficult, which is precisely the escape hatch principle 5 refuses.
A clause that is merely unimplemented is `unreviewed`, `partial` or `reported` — never
`out-of-scope`.

The rest of the vocabulary exists to keep six different situations from wearing one word:
the project *choosing* for a whole clause (`out-of-scope`), the project *choosing* for one
sentence inside a clause every other requirement of which is executed (`departed`), the
project *not knowing* (`unreviewed`), the project *owing out loud* (`reported`, and `partial`
for part of a clause), the project *owing in silence* (`silent`), and the requirement having
no meaning for a screen (`inapplicable`). `out-of-scope`, `departed` and `inapplicable` are
permanent; **`writer-side` is not** — a clause that
addressed only a generator becomes this tree's the moment it grows one, and this tree has grown
two. The remaining four are different
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
`doc/md/ISO_32000-2_sponsored_EC3.md`, which is **not tracked in the clear** — ISO's text is free
to obtain and not free to redistribute, so it lives inside `doc/specifications.zip` and a developer
unpacks it (ADR 0187). It has no skip path all the same,
and unlike the pdf.js submodule that is deliberate: a missing corpus costs a ratchet, and a
missing standard costs every citation in the tree its only check. It:

- builds the clause index from the file's own `##` headings, each giving a clause number, a title
  and a line range;
- fails on a `§` citation in Rust source naming a clause the standard does not have;
- fails on a rustdoc blockquote whose text does not occur within its cited clause's range,
  compared with whitespace collapsed and `![Image]` lines skipped;
- fails on a ledger row whose clause does not exist, or which claims `implemented` without
  naming a code site and a test that exist;
- fails on an `out-of-scope` row whose `exclusion` is not one of principle 5's closed
  entries — the constraint that keeps the status from becoming a graveyard;
- fails on a `departed` row whose note names no ADR — "decided against with its cost recorded"
  is a claim about a document somebody can open, so the row has to name the one that argued and
  priced the departure, calibrated by a plant (trap 13);
- fails on a `partial` or `reported` row that [`doc/todo/65`](todo/65-the-remaining-frontier.md)
  does not place exactly once, and on a row the map places that is in neither status — both sides
  are lists of clause numbers, so a disagreement is a fact rather than a reading
  (`conformance::frontier`);
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

**The crate's other test files gate the repository's own prose and tools rather than the
standard**, because that is the crate whose gates already read the repository's files: a Rust path
a doc comment names is one the tree declares (`tests/names.rs`, ADR 1273), a record is at most forty
lines (`tests/records.rs`), `--bin cited`'s rank is calibrated by a planted pair (`tests/cited.rs`,
ADR 1274), and `tools/batch.sh commit` stages the whole population by name while `close` refuses a
worktree holding uncommitted work (`tests/batch.rs`, ADR 1313). The sweeps under `src/bin/` —
`pointers`, `overtaken`, `retired`, `unread`, `cited` and the rest — are reading lists and never
gates, since each judges prose; `tools/state.sh` runs them by section.

Two ratchets, both in the gate and both two-directional. `UNREVIEWED_CEILING` may only fall.
`REVIEW_OWED` names the clauses the code cites whose rows are still `unreviewed`, and a clause not
on the list fails immediately, while a clause on it that *has* been reviewed must be deleted from
it. **It is a list rather than a count**, because filling rows in one sitting to make a number go
down is exactly the rubber stamp the ledger exists to prevent.

On quoting the standard: `doc/md/` is **not** committed — `/doc/*.pdf` and `/doc/md` are ignored
and only the encrypted `doc/specifications.zip` is tracked (ADR 0187) — so a quotation in a source
file is the one copy of ISO's words this repository carries in the clear, and it travels with any
code later published or excerpted. Keep them to the load-bearing sentence, which is also the right
length for readability. **And this permission is ISO 32000-2's alone**: every other specification
text under `doc/md/` is held as licensed to a single reader and is cited and paraphrased, never
quoted, with the three ETSI texts stricter still — their notice permits no reproduction in any form,
so nothing of them appears between quotation marks or after a `>` (ADR 1085).
[`doc/third-party-data.md`](third-party-data.md) states the position per text.

**How it gets filled.** By clause family, from ordinary work — all four subclauses of §8.9.6
while implementing image `/Mask`, all of §8.11 while implementing optional content. A family is the
right unit because that is how the standard distributes its requirements, and because §9.6.5.4 was
missed for the opposite reason: nobody had read §9.6.5 as a unit. A one-pass review of the whole
ledger is the kind of task that is abandoned partway and afterwards remembered as complete.

**The gate is built, and `cargo test -p conformance` is it.** ADR 0016 has the whole argument,
including what was decided against; `tools/state.sh conformance` prints where it stands — the
citations, the quotations, the distinct tables and the rows — because a table of those figures in
this file is exactly what lets a round write "unchanged" without running anything.

**Each of the four checks was confirmed to fail when its defect is put back**, which is trap 13
applied to the gate itself. The measurement that justified building it found two citations naming
clauses that do not exist and three of five sampled quotations that were paraphrases inside
quotation marks.

**What reading a clause family produces is the thing no automated check can.** Three shapes recur,
and all three are arguments for the method in *How it gets filled* above:

- **A wrong number the standard happens to have passes every check there is.** `/SMaskInData`
  cited §8.9.5.4 — a real clause, about alternate images, and not the one that defines the entry.
  Only a person reading the clause catches that, and reading §8.9.5.4 properly then produced a
  silence of its own: a base image hidden by `/OC` should be replaced by its first visible
  alternate.
- **The clause that looks like a formality produces most.** §8.6.8 is a table of twelve operators
  everybody knows are implemented, and reading it gave three findings — its restriction on colour
  operators governs uncoloured *tiling patterns* as well as `d1` glyph descriptions, its list is
  not what Table 111's parenthesis implies, and `cs`/`CS` must set an initial colour which is black
  in only three of its six cases. None reported anything at runtime.
- **One gap gets a row in every clause it is a gap for.** §11.7.4's overprinting is six rows;
  §11.4.6, §11.6.6 and §11.3.7.3 are the transparency-group gap recorded three times. A reader of
  any of them should find it, which is what a ledger is for.

**Table numbers are now checked, weakly and honestly.** The tree cited "§9.3.6 Table 106" for
the text rendering modes in four comments, two tests and a written report; the modes are Table
104, and Table 106 is the text-*positioning* operators. Every automated check passed, because
the clause exists and the table exists and only the pair is wrong. The obvious gate — the
clause beside a table reference must be one the standard discusses that table in — was built
and then **rejected by its own output**: it fails fourteen of this tree's twenty-five
references and all fourteen are correct writing, because a comment about one clause routinely
names a table belonging to another. A gate that is more exceptions than rule is not a gate. So
the assertion is the weaker true one, that the number names a table the standard has, and the
gate *prints the title of every distinct table the tree cites*, one line each, in which
"Table 106 — Text-positioning operators" beside a file about rendering modes is visible at a
glance. A checker cannot read a comment's intent; a person reading that list can.

## 6. Security architecture

Memory safety is necessary, not sufficient.

**What is built:**

- **The image codecs are confined.** `pdf-sandbox` confines a worker process with resource limits,
  Landlock and seccomp-BPF, and JBIG2, JPEG 2000 and CCITT decode in it. The worker receives one
  image's bytes over a pipe and returns samples over another; it never learns which document they
  came from. `--no-sandbox` decodes in process instead, which is a supported choice for trusted
  documents and prints what it gives up. ADR 0014.
- **The document, the interpreter and the rasteriser are confined too**, in a second worker:
  `viewer-confined`'s `pdf-view-worker` confines itself before it reads a byte and speaks
  `viewer-core`'s own vocabulary over a pipe, and `quorra-confined` is a window built on it.
  `doc/state-of-play.md` has what crosses and what it cost.
- **Explicit memory and time budgets** against decompression bombs and pathological content, since
  Rust prevents corruption and not exhaustion.

**What is not, and is deliberately kept in view:**

- **The flagship window is still in process.** Moving `viewer-ui` to the confined tier is a change
  of tier rather than a switch; the number it was waiting on exists (ADR 0597), so what is left is
  the move.
- **GPU-touching code ideally gets its own process** — drivers are large bodies of unsafe C and are
  themselves an attack surface.
- **AcroForm JavaScript, if it is ever supported, is a separate sandboxing problem.** Deferred by
  `CLAUDE.md`'s exclusion list, not designed out.

**No C or C++ library reaches a document's bytes**; the confined codecs are pure Rust, and the
confinement is for panic containment and a memory ceiling rather than for containing C. `ring`,
under the host's submission client, is the one C library a host process links for a job of its
own, and section 1 above says what reaches it (ADR 1291, `doc/questions/Q130`). `pdf-sandbox` is
`#![forbid(unsafe_code)]` over `landlock`, `seccompiler`, `rustix` and `libc` for the system-call
numbers — all four expose safe interfaces.

## 7. Environment

**The machine, the agent's account and the display are [`doc/environment.md`](environment.md)'s**,
together with the working agreements and the one command a fresh clone needs. A second copy of an
installed version is a number nothing checks — `pacman -Q` is what counts them — so what stays here
is the *policy* about dependencies rather than the inventory of them.

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

**`vulkan-swrast` matters more than it looks**: it makes GPU output reproducible in CI, so visual
diffs do not go flaky on a driver update. `vulkaninfo --summary` says which adapters this machine
offers.

**KDE Frameworks 6 on Arch has no `kf6-` prefix** — the packages `kio/`'s worker builds against are
`kio`, `kconfig` and `ki18n`, and `doc/stack.md` prices that face's toolchain.

**Wanted and absent**: `pdfium` (AUR), which would be the fourth voting reference renderer.

## 8. Open questions

Questions the project owner has to settle are one file each in
[`doc/questions/`](questions/) — `Q*.md`, answered by an `A*.md` of the same name — and
[`doc/todo/README.md`](todo/README.md) indexes the work this program owes. What is left here is the
two infrastructure questions neither of those covers:

- **How far a generated predicate should be evaluated.** Arlington's `SpecialCase` and its
  predicate-bearing `PossibleValues` are carried into `pdf-spec` verbatim and unevaluated (§5),
  because an evaluator needs a document to evaluate against. What is undecided is whether the
  evaluator is worth building at all, and against which reader.
- **Acrobat golden-set capture process.** Where all open-source renderers are jointly wrong,
  Acrobat is the gold standard, it is not scriptable on Linux, and so a small manually-captured set
  is the only route. Nothing has captured one.
