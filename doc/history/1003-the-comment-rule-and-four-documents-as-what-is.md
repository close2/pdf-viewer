# 1003 — The comment rule, and four navigational documents rewritten as what is

Date: 2026-09-12. ADR: 1023. Worktree `/home/AI/pdf-viewer-rounds`, branch `batch-999-1004`, on
`af11dd09` (`main`), with five sibling rounds live in the same tree. A first attempt at this round
was cut off by a quota limit before it did anything.

Files: `CLAUDE.md` (one paragraph appended to *Where knowledge lives*), `doc/PLAN.md`,
`doc/crate-map.md` (the header prose only — session 1000 owns the rows), `doc/state-of-play.md`,
`doc/HANDOVER.md`, `doc/adr/1023`, this file.

`doc/reviews/984-direction-and-boundaries.md` Finding 2 is the brief and ADR 1005 §2 the proposal.
ADR 1023 carries the rule, the classification of every removal under ADR 0974's four headings, and
the design of the Rust-comment sweep this round deliberately did not run.

**This file is where the removed prose went.** Nothing below was deleted from the project: each
block is what one of the four documents said on the date above, kept verbatim so that a reader of
the diff can tell a move from an edit (ADR 0232), with a line saying what stands in its place.
A record keeps its chronology; that is why these sentences are here and not there.

## `doc/PLAN.md` §1 — the stack table

**Replaced by** a pointer to `doc/stack.md`, which is the table `CLAUDE.md` moved out of itself and
which `doc/stack.md`'s own header already says this section is the *rationale* for. Two documents
stating one choice is how they drift, and these two had: `doc/PLAN.md` said "CPU first, GPU behind
a trait" where `doc/stack.md` says "GPU first, for page one and every page after it", which is the
owner's decision and the one principle 2 states. The row for dialogs was wrong in both files: no
crate in this tree depends on `ashpd` (`cargo metadata`), `crates/viewer-ui` has no dialog module,
and the decision §12.7.6.4 actually puts on a host is `viewer-host`'s import policy. `doc/stack.md`
is session 1000's file this round and still carries that row.

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

## `doc/PLAN.md` §1 — why Qt was dropped

**Replaced by** three paragraphs of what is: the flagship window is `winit` and this tree's own
drawing with no FFI on the launch path; Qt is in the tree as `crates/viewer-qt`, a host on
`viewer-core`'s boundary and the only C++ here (ADR 0246), beside `crates/viewer-gtk` (ADR 0244);
and no crate depends on `ashpd`. The review named this paragraph as a document contradicting the
tree — the crate has existed since the four-hundred-and-tenth session and this sentence said the
toolkit was dropped.

**Why Qt was dropped.** Qt was justified by native KDE file dialogs and accessibility.
Neither holds: `xdg-desktop-portal-kde` is installed, so *any* toolkit gets native KDE
dialogs through the portal via `ashpd`; and `AccessKit` provides AT-SPI accessibility for
custom-drawn Rust UIs. That removes the justification for the `cxx-qt` bridge, moc, and an
eventual CMake/Corrosion migration — the most fragile part of the build. A pure-Rust stack
with no FFI boundary is also materially better against principle 4 (exemplary code): the
whole stack reads in one language.

Given up: a mature widget set if the UI grows beyond a viewer, and Qt's free
i18n/menu/shortcut infrastructure. Revisit if AcroForm editing UI becomes a goal.

## `doc/PLAN.md` §1 — the image-codec paragraph, and its appended correction

**Replaced by** the two facts and no history: the decoders are pure Rust and taken with
`default-features = false`, and the sandbox is for panic containment and a memory ceiling rather
than for containing C, of which this tree has none. The FORCEDENTRY anchor stayed, because it is
the evidence for *which* codecs run confined.

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

## `doc/PLAN.md` §1 — the `rustybuzz` exclusion

**Replaced by** nothing here: `doc/stack.md` owns it, at greater length and with ADR 0348's
reading of §12.7.4.3's one Arabic witness, and `CLAUDE.md`'s own table sends a round asking about
a dependency there. The shorter copy below had already lost the part that matters — that no
compiled-in face has one Arabic glyph, so the shaper question is moot until a glyph source exists.

**`rustybuzz` is deliberately excluded.** PDF content streams carry already-positioned
glyphs — the producer shaped them at authoring time. Re-shaping would move glyphs away
from where the document specifies, breaking fidelity precisely on complex-script
documents. Reconsider only for text we generate ourselves.

## `doc/PLAN.md` §1 — "Why CPU first"

**Replaced by** "Why there is a CPU backend at all, and why it was built first", which keeps the
same-scene-oracle argument and the 0.0136/255 measurement, adds the refusal fallback, and says
outright what the heading had come to contradict: the CPU backend is not the startup path, because
page one goes to the graphics device by the owner's decision.

## `doc/PLAN.md` §2 — the ten-crate tree drawing

**Replaced by** the rule and the command: the workspace members are `crates/*`, `tools/*` and
`raster/crates/*`, `cargo metadata --no-deps` is the authority, and `doc/crate-map.md` is the map.
The drawing listed ten crates against a workspace of thirty-seven packages, named `tools/corpus/`
which has never existed, and gave `render-cpu` the startup path, which the owner's page-one
decision took away from it. The `forbid(unsafe_code)` paragraph stayed and gained the three crates
that only `deny` it, read off `viewer-qt`'s
`only_the_three_named_crates_in_the_tree_lift_the_denial` rather than written down.

```
quorra/
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

## `doc/PLAN.md` §3 — the phases, their checkboxes and the spike reports

**Replaced by** *Foundation, the build, and the test layers*: what the foundation is, what the
crate graph is, what the test layers are, what the viewer is, and a short list of what the five
de-risking spikes settled that is still a property something asserts. The conformance-gate bullet
said "Not built" while §5a of the same file said it was built in the ninth session; it now says
`cargo test -p conformance` runs it. The per-spike test counts went as counted facts, the
`~~strikethrough~~ **Done.**` markers as session narrative.

One consequence to record: `doc/questions/Q25` cites this file's "Phase 4 — Test layers" heading by
name, and that heading is now "The test layers" in the same section with the same material. Q25 and
A25 are both records and are not edited.

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

`cargo run --release -p viewer-ui --bin quorra -- document.pdf` opens a real file.
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

## `doc/PLAN.md` §4 — the oracle's figures, the goldens correction, the hayro speed table

**Replaced by**: the oracle paragraph keeps its rule (the bound is twice the references' own
disagreement on that page) and hands its figures to `tools/state.sh oracle`; the goldens section
states what the self-golden *is* — `crates/pdf-model/tests/raster_golden.rs`, session 996's, ADR
1016 — and states the shared-across-adapters property without the "correction to an earlier
assumption" framing; and the speed table goes to `doc/performance.md` §4, which owns the
measurement, keeps a column per time it was taken, and carries the same lesson in a form that has
not drifted. The table below had: aggregate 5.9× where `doc/performance.md` says 6.9×, a median of
1.62× where it says 2.15×, and a pointer to "Where the time went" in `doc/HANDOVER.md`, a section
that file has not had since it was split.

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

**Correction to an earlier assumption here.** This originally said RADV and lavapipe
would not produce identical pixels, so goldens had to be per-backend. Measurement showed
the opposite: for the vector path they are byte-identical, because Vello's compute
pipeline has no driver-dependent fixed-function rasterisation. Goldens can therefore be
shared across adapters, and a test pins that property so its loss is noticed. Checked on
one vendor and simple scenes only — text and images may still diverge. See ADR 0004.

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

## `doc/PLAN.md` §5a — the ledger's first green run and the session-by-session reviews

**Replaced by** four paragraphs: the gate is built and `cargo test -p conformance` is it, with
`tools/state.sh conformance` printing where it stands; the four checks were each confirmed to fail
when their defect is put back (trap 13); and three recurring shapes of what reading a clause family
produces — a wrong number the standard happens to have, the clause that looks like a formality, and
one gap getting a row in every clause it is a gap for. The findings themselves are in the ledger
rows and in ADRs 0016 and 0022; what came out is the chronology they were indexed to and the
progression of counts, which is ADR 0281's own example of a table that lets a round write
"unchanged" without running anything.

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

## `doc/PLAN.md` §7 — the machine, the packages and the caveats

**Replaced by** a pointer to `doc/environment.md`, which owns the machine, the account, the display
and the working agreements; the dependency *policy* notes stayed. "Not yet a git repository" was
false — `git rev-parse --is-inside-work-tree` says `true`, and every ADR in the tree is a commit.
The `sudo -u AI` and X-authority caveat is `doc/environment.md:243-275`, word for word the same
claim.

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

## `doc/PLAN.md` §8 — three settled open questions

**Replaced by** a pointer to `doc/questions/` and `doc/todo/README.md`, plus the two infrastructure
questions neither covers: how far a generated Arlington predicate should be evaluated (still open —
`pdf_spec::model` carries `SpecialCase` "verbatim and unevaluated"), and the Acrobat golden-set
capture process. The three removed are settled by the tree: `vello_cpu` versus `tiny-skia` for
backend #1 was decided by `render-cpu` being the oracle and `render-raster` the third rasteriser;
Type1 is implemented directly in `crates/pdf-font/src/type1.rs`; and the `SpecialCase` bullet was
re-added rather than dropped once the code was read.

## 8. Open questions

- Extent of Arlington `SpecialCase` predicate support in codegen (Phase 5C).
- `vello_cpu` feature coverage vs falling back to `tiny-skia` for backend #1.
- Acrobat golden-set capture process.
- Type1 font strategy: convert to CFF, or implement directly.

## `doc/state-of-play.md` — four corrections appended to the sentences they correct

Each is replaced by the current sentence alone. The arguments are in the ADRs beside them and
nothing else moved; the file's structure was deliberately kept, as ADR 0974 kept it.

1. The article-threads panel: **"and this sentence said 'not one corpus document' for as long as
   the panel has existed"**. The population lesson is ADR 0405's and stays in the sentence ("which
   population a claim is about is part of the claim"); the account of the sentence's own history
   went.
2. `viewer-qt`'s `unsafe` token: **"This sentence read 'one hand-written `unsafe` token in the
   tree' and 'no other crate lifts the denial', and both were widenings of a true claim about one
   crate … the test is named for its own length and the name moved twice while this sentence said
   *no other*"**. What stays is the claim being about this crate rather than the tree, and the test
   name that is the list.
3. The C ABI's vocabulary: **"What that round could then claim — that the entry points *are* the
   whole vocabulary — has decayed and is counted rather than repeated"**. What stays is that
   `tools/state.sh hosts` counts it (ADR 0509).
4. The password field: **"The freeze's three amendments came first and one of them was a bug … this
   program wrote a person's typed password into the file it saved … and no longer does"**. What
   stays is that `save` writes neither the value nor the appearance and reports each one withheld,
   under Table 231 bit 14's NOTE. ADR 0247 has the incident, in full, where a record belongs.

Two smaller ones: the header's "which is why it is no longer in `doc/HANDOVER.md`" became "which is
why `doc/HANDOVER.md` points at it rather than holding it", and "was owed for a long time" came off
the sentence about the viewer being used.

## `doc/HANDOVER.md` — two sentences about the file's own past

"everything it used to state in full now lives one hop away" became "everything it points at is
stated in full one hop away". **"This file has been halved five times"** came off — a counted fact,
and one no command prints — and the sentence that carried it now points at `CLAUDE.md`'s comment
rule as well as at ADRs 0232, 0281 and 0428. "what used to be this file's 'Things worth knowing'"
became a plain statement of what each group file carries.

## `doc/crate-map.md` — the header only

Session 1000 owns the rows this round and added `pdf-signature` and `pdf-archive` and corrected
`render-quorra` to `render-raster`. What this round wrote is two paragraphs of header: that the
population is `cargo metadata --no-deps`'s rather than this file's, and that a row says what a
crate is responsible for in the present tense, with `grep -c 'New in the'` naming how much of the
chronology is still in the Notes column. The rows themselves are a later round's, by the sweep
design in ADR 1023.
