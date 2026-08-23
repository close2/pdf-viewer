# Traps: instruments, reports, and reading what a gate prints

Status: **standing** — each is a mistake somebody actually made in this tree.
Read by: a round that runs a gate, believes a number, adds a report, sweeps the tree for a class of
defect, or adds a lint exception.
`doc/todo/02-every-round.md` §2 owns the gate sequence; this file is what the gates and their
numbers do wrong.

`doc/HANDOVER.md` is the index and names which group holds which trap. **Every trap keeps its
number**, because `crates/`, `tools/`, `doc/conformance/ledger.toml` and dozens of ADRs cite them
by number and an ADR is not edited to follow a file that moved underneath it (ADR 0232 §2).

## How to read what `tools/state.sh` prints

**The numbers are not in any document** — `tools/state.sh` prints them, and that is deliberate
rather than tidy: a round told to *measure* something must not be able to read the answer in a
document, because a table of gate figures is exactly what lets a round write "unchanged" without
running anything (ADR 0281).

- **Counts are ratcheted**: they may only improve, except where a rise is a *new report* and is
  written down as one (trap 5).
- **A gate's own number, never arithmetic beside it.** This project has twice carried a sum that
  was stale while the gate figure two lines above it was current, which is why `state.sh` filters
  a gate's output and adds nothing up. If you need a total, print it.

## Traps

### 7. `#[expect]`, never `#[allow]`

Every lint exception is `#[expect(..., reason = "...")]`. It errors when it stops being necessary,
which has already removed several. A bare `allow` hides that forever.

### 10. The sandbox worker is a separate binary, and Cargo will not rebuild it for you

`cargo test -p pdf-model` builds pdf-sandbox's *library*, not its `pdf-sandbox-worker` binary —
Cargo never builds another package's binaries. So the tests run against whatever worker was last
compiled. Not hypothetical: the seventh session inverted the black-and-white sense of every JBIG2
sample and the test passed. `cargo test --workspace` or `cargo build -p pdf-sandbox --bins` builds
it. Both gates fail loudly if the worker is *missing* — and a missing worker and a stale one look
nothing alike.

**And there are now two of these, in a profile of their own.** Since the three-hundred-and-eighty-fifth
the corpus gates run under `--profile gates`, so the worker they spawn is `target/gates/pdf-sandbox-worker`
and not the release one; and `pdfref-hayro`, which the oracle spawns for a fourth reading, is a second
program under the same rule. That one is worse than trap 10's original shape rather than better: it
**fails silently**. `Reference::Hayro` votes on nothing, so its absence leaves every verdict intact and
only removes a picture — which is how it went unbuilt by `doc/todo/02` §2 for its whole life and was
noticed by a reference-render count falling 861 with nothing else moving (ADR 0222). Both are lines in
§2 now, and the tell is the same one trap 10a names: the hit rate.

### 10b. A *new module file* is a fifth thing Cargo will hand you stale

Adding `crates/pdf-render/src/medium.rs` in the six-hundred-and-eleventh session left the
**release-profile** fingerprint of every crate above it unaware that the file existed: editing it
and running `cargo build --release -p pdf-model --example …` printed `Finished` in 0.10 s,
recompiled nothing, and ran the *previous* revision's binary. `cargo build --release -p pdf-render`
alone did rebuild the crate, and the build that depended on it still reported it `Fresh`.

It cost this round its central measurement twice. The claim being made was that no corpus pixel
moved; the first two runs of `examples/raster_digest` said so against a binary that did not contain
the change, and the calibration that was supposed to prove the instrument could fail — moving
`Medium::PAGE_ONLY`'s colour off white — reported **no difference either**, which is what finally
gave it away. An instrument that cannot fail has not been shown to work, and neither has one whose
inputs were not rebuilt.

The fix is one word: `touch` each changed crate's `src/lib.rs` before either arm of a two-revision
comparison, and before any measurement taken from a release binary in the same sitting as a source
edit. The tell is a `Finished` with no `Compiling` line after an edit you know you made; `-v` prints
`Fresh <crate>` for the crate you just changed, which is the sentence to disbelieve.

### 15. A sweep binary carries its tree with it, so one from a neighbour's build directory measures the neighbour

Traps 10, 10a and 10b are all about an instrument being *stale*. This one is not stale at all: it is
current, it runs, it prints a plausible number, and the number is about **another worktree**.

`tools/conformance`'s `root()` is `Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")`, which is
baked in at compile time and does not move with the working directory. So every sweep — `counts`,
`owed`, `pointers`, `quotations`, `tables`, all of them — reads the ledger and the `SOURCE_ROOTS` of
the tree it was *built from*, whatever directory it is invoked in. The same holds for
`examples/absence_audit`'s `corpus`, which resolves its populations the same way.

That is harmless in a single tree and is a trap the moment rounds run in parallel, because each
worktree has its own `target-dir` in `.cargo/config.toml` and the *main* tree's is the one whose
path a round remembers. The six-hundred-and-seventy-sixth session ran its whole before-sweep from
`/home/AI/cargo-target/pdf-viewer/release`, got thirteen summary lines that looked exactly right,
and had measured `/home/cl/projects/pdf-viewer` — a tree it had not edited. The tell is that
*nothing moves* when you re-run after an edit, which reads as "my change touched no sweep" and is
the most comfortable possible wrong answer.

Two rules:

- **Take the path from the toolchain, not from memory**: `cargo run --release -p conformance --bin
  <name>` is what `doc/todo/01` states, and it cannot pick the wrong tree. If you invoke a binary
  directly, get its directory from `cargo metadata --format-version 1 --no-deps`.
- **A before/after comparison needs a before that was compiled from the "before" tree.** `git stash`
  is forbidden here (the stack is shared), so export it — `git archive HEAD | tar -x -C <dir>` — and
  build the sweeps *inside* the export with its own `target-dir`. The export carries only tracked
  files, so symlink `doc/md`, the specification PDFs, the submodules and `corpus-cache` back in
  first; without `doc/md` three sweeps refuse outright, and without the rest `pointers` reports
  dozens of live paths as "not carried" and the comparison invents deltas the round did not cause.

### 16. A *gate's* answer can depend on **how much of the workspace was built**

> **The heading used to say "which build directory it was compiled in", and the
> six-hundred-and-ninety-eighth session established that the directory is a symptom.** The variable
> is **build scope**, and the correction is kept below the original account rather than replacing it,
> because the original is what a round will recognise when it happens again.

Trap 15 is a sweep binary that carries the wrong **tree**. This is the same family one step further
in, and it is worse because the instrument is a **ratchet**: the same sources, built in two
different `target-dir`s, gave two different numbers, and one of them passed the gate.

The six-hundred-and-ninety-fifth session found the accessibility census failing —
`elements placed by their own marks: 93258, and it was 93267` — and checked whether the round had
caused it, by adding a scratch `git worktree` at its own branch point (`main`'s HEAD, unmodified)
and running the census there. **It printed 93267 and passed.** Which would have said the round's
change had moved nine elements — except that the scratch worktree had no `target-dir` of its own and
so had built into `/home/AI/cargo-target/pdf-viewer`, the *main* tree's. Re-run with
`CARGO_TARGET_DIR` pointed at an empty directory, the same worktree at the same commit printed
**93258 and failed**, which is what the round's own tree printed. Three runs each way, deterministic
both ways.

So: the ratchet had been broken on `main` for some time — at least back to the merge of round 686,
which is as far as this session bisected — and the shared directory was answering with something
that no from-scratch build of those sources produces. **What the mechanism is was not established**,
and saying so is the point: `pdf-spec/build.rs` does emit `cargo:rerun-if-changed` for the Arlington
TSVs, `main`'s working tree and its submodules were clean, and no `RUSTFLAGS` differed. It is
recorded as an *observation with a command behind it* rather than as a diagnosis.

Two rules, and the first is the cheap one:

- **A round that suspects its own change of moving a gate compares against a build of the branch
  point in a directory of its own.** `git worktree add <scratch> <base>` plus
  `CARGO_TARGET_DIR=<empty dir>` — without the second half the comparison is worth nothing, and it
  is exactly the half that is easy to forget because a worktree *usually* inherits a `target-dir`
  from `.cargo/config.toml` and a scratch one has none.
- **A gate that fails is not a gate to argue with from a document.** The census floors are not
  written anywhere but in the test, which is why this was caught at all; had the number been
  recorded in a note somewhere, "unchanged" was available for free (ADR 0281's whole argument).

#### What the mechanism turned out to be — the six-hundred-and-ninety-eighth

Four readings of one commit, deterministic twice each, the two test binaries carrying different
digests:

| | reads | verdict |
|---|---|---|
| the shared build directory | **1336** | passes |
| a clean directory, **subset** built | **1345** | **fails** |
| a clean directory, after `cargo clean -p` on four crates | 1336 | passes |
| a clean directory, **whole workspace** built | **1336** | passes |

So it is not the directory and it is not staleness: **Cargo unifies features across whatever is in
the build**, and a dependency built for `viewer-core`'s subset gets a different feature set than the
same dependency built for the workspace — and the program then counts nine structure elements
differently. Building the whole workspace in the *clean* directory reproduces the shared directory's
answer exactly.

**This makes the six-hundred-and-sixtieth session's report real, and this project's record of it
wrong.** That round said `cargo nextest run -p pdf-model` *alone* fails six CCITT tests where
`--workspace` passes, and blamed feature unification resolving `hayro-ccitt` differently under a
scoped build. The six-hundred-and-sixty-fourth checked it on merged `main`, got 1099 passing, and
recorded it as **not reproducing** — in a fully-built shared directory, which is exactly the scope
where the defect hides. The rule is general and it is the trap's real lesson:

> **A claim that a defect does not reproduce is a claim about the conditions you reproduced it
> under.** Name the conditions, or the claim is worth what an unnamed population is worth.

Three things follow and none is settled: `cargo build --release --bin pdf-viewer` is a **third**
scope and nothing has established which feature set the shipped binary carries; **which of 1345 and
1336 is right is unknown**, and the ratchet's floor was set under one of them without anybody asking
which; and **a dependency feature that changes what the program computes should be stated by this
workspace rather than inherited** from whichever crates happen to be in the build. That is a round's
work, and `doc/HANDOVER.md` names it as one.

### 10a. A cached reference render is a fourth thing that can be stale

The key is built from the invocation itself plus the renderer's version and the document's
SHA-256, so **a flag not in the key is a flag not passed to the renderer either**. What it cannot
see is a renderer whose output changes while its version string does not. **The variable names a
*directory* and only the literal `off` disables it** — `PDFREF_CACHE=on` silently starts a fresh
319 MB cache in a directory called `on`. **The hit rate is printed and it is the tell**: under 99%
on an unchanged tree means the corpus or a renderer moved. A remembered *timeout* is the one entry
whose truth decays, counted separately and expiring after a week.

### 11. A report is only as good as the condition it fires on

Trap 5's other edge. The reflex is to report whenever the unimplemented thing *could* be involved.
Four instances: §9.3.8's text knockout named 7 documents on one of the clause's two conditions and
took **three agreeing pages out of the gated set**; §11.6.2 named six, three of which set an alpha
to *zero* so there are no two portions to composite; §11.7.4's overprinting was 63 documents and
six `silent` rows and the honest condition has **no members** on this device; §12.5.6.19 fired
where the clause asks for nothing at all, naming 23 documents.

**Derive the condition from the clause, print what it matched before trusting the count, and cost
it in gated pages** — a page that reports is a page the oracle stops judging. **Both of §9.3.8's
conditions outlived the report**: they are what decides whether the implementation builds a group.
A condition worked out for a report is worth keeping when the feature lands. And the reverse worry
is real: **a report can hide another report** — `knockout_smask.pdf`'s knockout gap was covered by
its soft-mask report for four sessions.

**And the trap has a fifth instance with its sign reversed, which the four above cannot warn
about: a condition narrowed by an exemption written for something else.** The report that names a
font drawing nothing of what it was asked to show is gated on a count of codes that reached no
glyph, and that count excluded a code §9.10.2 could not *name* — an exemption whose argument is
sound and is about the **reader**, applied to a question about whether the **program** answered. So
`issue17333.pdf`, whose one code an embedded font's `cmap` does not cover, drew a wholly blank
sheet with `unsupported: []`, and every instrument that measures the picture read zero
(ADR 0520). Two rules come out of it and neither is about deriving the condition from the clause,
because this one was: **an exemption is part of the condition and needs the same evidence** — write
down which question it answers, because a second question arriving later will inherit it silently;
and **a report built out of a count inherits every one of that count's exclusions**, so a count's
doc comment naming two of its three is a defect in the report and not only in the prose.

### 13. A sweep for a defect must be run against the defect before it is believed

A round told to look for a class of defect writes a grep, gets a handful of hits, reads them and
reports the tree clean. **That is a measurement with an instrument nobody calibrated**, and the
six-hundred-and-fourth session calibrated one and watched it fail.

The class was ADR 0438's: a byte string made into text by a lossy route and then used to *decide*
something. The obvious sweep looks for the conversion — `from_utf8_lossy` inside a `get`, a `==`, a
`match`. Run against a scratch copy of the very files the defect had lived in, at the revision
before the fix, **it prints nothing**: the conversion was in one function and the lookup was in
another, and no grep over a single line joins two functions.

The sweep that worked inverted the question and looked at the *decision* — every dictionary lookup
whose key is not a string literal — because ISO's own keys are literals in this source, so a key
that is not one came out of a file. That sweep names the planted defect five times, and it found
six more sites the first one could not see.

So: **plant the defect back and confirm the sweep names it.** A scratch copy and `git show
<commit>^:<path>` is the whole cost, and without it "the sweep came back clean" is a sentence about
a grep rather than about the tree. ADR 0439 has both sweeps as commands.

**And the trap has a second shape, where the sweep is a census over documents rather than a grep
over source: a census derived from the *clause* is not a census of the *defect*, because the code
has conditions the clause does not.** `examples/operator_shape_census` counts ISO 32000-2
§8.5.2.1's error by lexing a page and finding an `l`, `c`, `v` or `y` keyword with no `m` or `re`
before it — which is exactly the clause — and named twelve documents and 5010 operators. The
interpreter asks one thing more: an operator only runs when its **operands parse as numbers**. On
`issue6342.pdf`, the one curated first page that census named, every offending `c` is preceded by
byte soup the lexer splits into keywords of its own, so not one of them ever reaches a path and the
page's display list has no such shape in it at all. The defect's true population over 1230 curated
first pages is **zero** (ADR 0563). Two rules out of it, and the second is the general one: **name
the population a census is about — the clause's shape or the program's behaviour — because they are
different populations and a row that quotes one for the other is stale the day it is written**; and
**a census that reaches for the clause when the interpreter is one call away has chosen the weaker
instrument**, since a report the code already raises can be counted directly.

## Things worth knowing

- **The sandbox is a flag and the default is the safe one.** `--no-sandbox` trades panic
  containment and a memory ceiling, not memory safety. There is deliberately no path that falls
  back to in-process decoding when the worker fails to start.
- **Debug builds are ~15× slower here**, and it changes what a test can assert: the corpus gate is
  2 s in release and minutes in debug. Run timing assertions in release and say so.
