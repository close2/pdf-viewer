# Traps: instruments, reports, and reading what a gate prints

Status: **standing** — each is a mistake somebody actually made in this tree.
Read by: a round that runs a gate, believes a number, adds a report, sweeps the tree for a class of
defect, or adds a lint exception.
`doc/todo/02-every-round.md` §2 owns the gate sequence; this file is what the gates and their
numbers do wrong.

[`README.md`](README.md) is the index: one line per trap saying the position that springs it and
the rule, which group holds which, and why every trap keeps its number.

## How to read what `tools/state.sh` prints

**The numbers are not in any document** — `tools/state.sh` prints them, for the reason `CLAUDE.md`'s
"Where knowledge lives" gives (ADR 0281).

- **Counts are ratcheted**: they may only improve, except where a rise is a *new report* and is
  written down as one (trap 5).
- **A gate's own number, never arithmetic beside it.** This project has twice carried a sum that
  was stale while the gate figure two lines above it was current, which is why `state.sh` filters
  a gate's output and adds nothing up. If you need a total, print it.

## Things worth knowing

- **The sandbox is a flag and the default is the safe one.** `--no-sandbox` trades panic
  containment and a memory ceiling, not memory safety. There is deliberately no path that falls
  back to in-process decoding when the worker fails to start.
- **Debug builds are much slower here**, and it changes what a test can assert: the corpus gate is
  seconds in release and minutes in debug. Run timing assertions in release and say so; the figure
  is measured, never quoted from a document (ADR 0428 §6).

## Traps

### 7. `#[expect]`, never `#[allow]` — now held by the compiler, on the part of it a compiler can hold

Every lint exception is `#[expect(..., reason = "...")]`. It errors when it stops being necessary,
which has already removed several. A bare `allow` hides that forever.

**A program holds this now, and the incident that put it here is why it had to.**
`clippy::allow_attributes` is `warn` in `[workspace.lints.clippy]` and in `fuzz/Cargo.toml`'s copy
of it, so under `doc/todo/02` §2's tier-1 `RUSTFLAGS="-D warnings"` a bare `#[allow]` is a build
failure rather than a rule a round has to remember (ADR 1042). The lint's own description is the
trap's sentence: "`#[allow]` will not trigger if a warning isn't found. `#[expect]` triggers if
there are no warnings."

**The prose stays because the rule was prose for 555 sessions and that was not enough.**
`doc/reviews/1018-what-retiring-a-trap-would-cost.md` counted eleven citations of it and three
committed violations in `crates/pdf-transform/src/archive/`, **one of which carried a `reason`** —
its author knew the convention and reached for the wrong attribute anyway. And the review's own
grep, which looked for `#[allow(clippy::` under `crates/` and `tools/`, found three of the 221
sites the lint found: `raster/`, the renderer folded in from its own repository, held 215 of them.
**A hand-written population is not the tree** (trap 25), and the instrument that answered correctly
was the compiler.

**What it cannot hold, which is now the only part a round owes by hand.** The lint fires on *outer*
attributes only, so a crate- or module-level `#![allow(...)]` — the form the census examples under
`crates/*/examples/` use — is not seen by it; `grep -rn '#!\[allow(' --include=*.rs crates tools
raster fuzz` lists those that stand. Calibrated rather than assumed (trap 13): an outer `#[allow]`
planted in `raster-pages` failed the clippy line by name — `error: #[allow] attribute found`, exit
101 — and an inner `#![allow]` planted in its place exited 0 and said nothing.

### 10. The sandbox worker is a separate binary, and Cargo will not rebuild it for you

`cargo test -p pdf-model` builds pdf-sandbox's *library*, not its `pdf-sandbox-worker` binary —
Cargo never builds another package's binaries. So the tests run against whatever worker was last
compiled. Not hypothetical: the seventh session inverted the black-and-white sense of every JBIG2
sample and the test passed. `cargo test --workspace` builds it, and a walk builds it as its
wrapper's own `--build '--profile gates -p pdf-sandbox --bins'` (or `--release`, the walk's own profile), inside its hold, so that it is
no older than the tree the walk reads (trap 109, ADR 1710). A missing worker and a stale one look
nothing alike.

**Only two of the eight corpus gates once failed loudly on a missing worker.** `pdf-model`'s `corpus` and `oracle` check; the accessibility
census, the selection census, `text_extraction`, `fixed_documents`, `jpeg2000` and
`render-raster`'s `corpus` did not — and the census's ratchet moved by nine structure elements
because of it, deterministically, for at least a dozen rounds while four rounds diagnosed it as
something else. That is trap 16, and it is trap 10 wearing another trap's clothes. All eight now
check, and `tools/conformance/tests/sandbox_gates.rs` fails a gate line in `doc/todo/02` §2 that
neither checks nor says why it needs none (ADR 0557).

**The other half of the lesson: `CCITTFaxDecode` travels this pipe too.** The paragraph above names
JBIG2, and §7.4.6's fax decoder is the third program on the far end of it beside §7.4.7's and
§7.4.9's. A round narrowing a test command to save time meets six CCITT tests that are red for no
reason in the document, which cost session 660 twenty minutes and cost session 664 an hour deciding
it was not real.

**And there are now two of these, in a profile of their own.** Since the three-hundred-and-eighty-fifth
the corpus gates run under `--profile gates`, so the worker they spawn is `target/gates/pdf-sandbox-worker`
and not the release one; and `pdfref-hayro`, which the oracle spawns for a fourth reading, is a second
program under the same rule. That one is worse than trap 10's original shape rather than better: it
**fails silently**. `Reference::Hayro` votes on nothing, so its absence leaves every verdict intact and
only removes a picture — which is how it went unbuilt by `doc/todo/02` §2 for its whole life and was
noticed by a reference-render count falling 861 with nothing else moving (ADR 0222). Both are lines in
§2 now, and the tell is the same one trap 10a names: the hit rate.

**And there is a *third* copy, which no `cargo build` line reaches at all** (ADR 0859, session 908).
`pdf_sandbox::worker_program` searches beside the running executable first and one directory up
second, and an **example** runs from `<target>/release/examples/` — a directory Cargo fills with
examples and never with the worker. So a copy left there by some earlier round is searched *ahead*
of the fresh one beside it, and neither `cargo build -p pdf-sandbox --bins` nor `doc/todo/02` §5
touches it: it is a hand-made file that ages on its own. Round 908 ranked a tracker's 166 documents
by ink with `examples/render_at` against a worker ten hours behind its own tree, and produced five
pages of −11 to −23 levels against both references that were the instrument rather than the files.
**The tell is the refusal's own sentence**, which names the stale path and both build hashes — so
`examples/open_one` on the head of any ranking is the check, and `cp <target>/release/pdf-sandbox-worker
<target>/release/examples/` is the fix.

The dev profile has a worker of its own. After a round's `--profile gates` builds, a plain
`cargo nextest run -p pdf-model` showed 38 decoder failures that read as a regression until
`cargo build -p pdf-sandbox --bins` had been run for `dev` as well (round 1414): the two profiles
keep two workers, and a test under one profile finds only that profile's. That build is a plain one,
because a crate-scoped test is no walk and takes no lock; a walk's is its `--build`.

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

`tools/conformance`'s `workspace_root()` is `Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")`, which is
baked in at compile time and does not move with the working directory. So every sweep — `counts`,
`owed`, `pointers`, `quotations`, `tables`, all of them — reads the ledger and the `SOURCE_ROOTS` of
the tree it was *built from*, whatever directory it is invoked in. The same holds for
`examples/absence_audit`'s `corpus`, which resolves its populations the same way.

That is harmless in a single tree and is a trap the moment rounds run in parallel, because each
worktree has its own `target-dir` in `.cargo/config.toml` and the *main* tree's is the one whose
path a round remembers. The six-hundred-and-seventy-sixth session ran its whole before-sweep from
`/home/AI/cargo-target/quorra/release`, got thirteen summary lines that looked exactly right,
and had measured `/home/cl/projects/pdf-viewer` — a tree it had not edited. The tell is that
*nothing moves* when you re-run after an edit, which reads as "my change touched no sweep" and is
the most comfortable possible wrong answer.

Two rules:

- **Take the path from the toolchain, not from memory**: `cargo run -p conformance --bin
  <name>` is what `doc/todo/01` states, and it cannot pick the wrong tree. If you invoke a binary
  directly, get its directory from `cargo metadata --format-version 1 --no-deps`.

**It is not only a sweep, and `doc/todo/02` §5 was the instruction that got it wrong.** That section
told a round to `install` from `/home/AI/cargo-target/quorra/release/`, a *literal* path, and in
a worktree round that is a **neighbour's** build directory — so the binaries a person picks up, and
every launch measurement taken from them, are another branch's. The seven-hundred-and-twenty-sixth
session spent three rebuild-and-run cycles on it: the GTK host was rebuilt each time, installed each
time, and each time ran the main tree's binary and printed nothing for a feature that was working.
The tell is the same one as above — *nothing moves when you re-run after an edit* — and the fix is
the same: derive the directory. §5 does now, `tools/round.sh` always did, and `tools/state.sh disk`
does since the same round.
- **A before/after comparison needs a before that was compiled from the "before" tree.** `git stash`
  is forbidden here (the stack is shared), so the before is an export — `tools/batch.sh export
  <session> --as <name>`, which links every gitignored path back in and builds under a target
  directory of its own (ADR 1890; traps 129, 162). A bare `git archive` carries only tracked files:
  without `doc/md` three sweeps refuse outright, and without the rest `pointers` reports dozens of
  live paths as "not carried" and the comparison invents deltas the round did not cause. A sweep
  that reads git's tracked files does not run in an export at all (trap 163).

### 16. A gate can measure a program **the build did not finish producing**

> **This heading has been wrong twice.** It said "which build directory it was compiled in", the
> six-hundred-and-ninety-eighth session established that the directory was a symptom and made it
> "how much of the workspace was built", and the seven-hundredth found that *that* was a symptom
> too. Both earlier accounts are kept below the answer, because they are what a round will
> recognise when this happens again — and because the sequence of three is the trap's real lesson.

Trap 15 is a sweep binary that carries the wrong **tree**. This is the same family one step further
in, and it is worse because the instrument is a **ratchet**: the same sources gave two different
numbers, and one of them passed the gate.

**The mechanism is trap 10.** `pdf-sandbox-worker` is a separate program; Cargo does not build
another package's binaries when it tests this one; and a build without it decodes **no**
`CCITTFaxDecode`, `JBIG2Decode` or `JPXDecode` image. Every other image draws, every document
opens, every page renders — and a count comes out that is about the build rather than about the
tree. Trap 10 has said the first half of that since the seventh session; what nobody had joined to
it is that **six of the eight corpus gates never checked.**

Measured with the conditions named, which is this trap's own rule: one commit, one `target-dir`
created empty for the purpose, built only by the census's own `cargo test … --no-run` line, **one
test binary of one digest, run twice each way**:

| the one binary, in the one directory | `placed by their own marks` | `with no place` | ratchet |
|---|---|---|---|
| no `pdf-sandbox-worker` beside it | 93 258 | 1345 | **fails** |
| the worker beside it | 93 267 | 1336 | passes |

The nine are `issue5481.pdf`'s, and nothing else in the corpus moves: it carries a `JPXDecode`
image, §14.8.3.3 derives a structure element's rectangle from what its marked content **drew**, and
an image that was refused drew nothing. So the two numbers are **not two readings of the standard**
— they are one reading by two programs, one of which is missing a component it ships with. No floor
moved. ADR 0557.

**The general rule, which is worth more than the instance:** two numbers out of one tree are not
automatically two interpretations of a clause. Ask what the two *programs* were first.

Three rules, and the first is the cheap one:

- **A round that suspects its own change of moving a gate compares against a build of the branch
  point in a directory of its own.** `git worktree add <scratch> <base>` plus
  `--target-dir <empty dir>` — without the second half the comparison is worth nothing.
- **A gate that fails is not a gate to argue with from a document.** The census floors are not
  written anywhere but in the test, which is why this was caught at all (ADR 0281's whole argument).
- **Name the conditions of every measurement.** *A claim that a defect does not reproduce is a
  claim about the conditions you reproduced it under.* Session 664 checked session 660's report in
  a fully-built shared directory — the one scope where a worker is always present — and recorded
  it as not reproducing.

#### What is now checked, and what is not

Every gate line in `doc/todo/02` §2 calls `require_the_sandbox()` or carries a line beginning
`// no sandbox worker:` with its reason, and `tools/conformance/tests/sandbox_gates.rs` reads §2's
own command block and fails on a gate that does neither. What it cannot see is a measurement that
is **not** a line in that sequence — an example, a benchmark, a figure a round takes by hand.

**Two of the six were already failing without the worker, in the wrong words**, which is worth
knowing because it is what a round will meet: `jpeg2000` failed with a list of documents that had
*stopped* differing from `OpenJPEG`, which reads as the decoder having improved.

#### The two earlier accounts, kept

**The six-hundred-and-ninety-fifth** found the census failing — `elements placed by their own
marks: 93258, and it was 93267` — and checked whether its own change had caused it by adding a
scratch `git worktree` at `main`'s unmodified HEAD. **It printed 93267 and passed** — because that
worktree had no `target-dir` of its own and built into the *shared* one, where a worker had been
built by some earlier round's §2 run. Re-run against an empty directory the same worktree at the
same commit printed **93258 and failed**. Three runs each way, deterministic both ways. It recorded
the observation without a diagnosis and **did not lower the floor**, which was right.

**The six-hundred-and-ninety-eighth** took four readings of one commit, deterministic twice each,
and found the two test binaries carried different digests:

| | reads | verdict |
|---|---|---|
| the shared build directory | **1336** | passes |
| a clean directory, **subset** built | **1345** | **fails** |
| a clean directory, after `cargo clean -p` on four crates | 1336 | passes |
| a clean directory, **whole workspace** built | **1336** | passes |

It concluded that the variable was build *scope* and that Cargo's feature unification was the
mechanism. The first half is right and the second is not — and the reason the table looks like
feature unification is that **every scope that produces a worker as a side effect also resolves
features differently**, so the two are confounded in all four rows. The digests differ for the
feature reason and the *counts* differ for the worker reason, and nothing in that table separates
them. What separates them is holding the binary fixed, which is the table at the top.

**The features were enumerated anyway**, because "we did not find one" is not an answer. Diffing
the resolved unit graphs three ways — the census's subset, the whole workspace, and
`--release --bin quorra` — the subset resolves `num-traits`, `once_cell`, `rustix`,
`linux-raw-sys`, `bytemuck`, `log`, `either`, `enumflags2`, `syn` and `proc-macro2` differently,
**and every one of the ten was traced to its consumer and changes no value the program computes**;
the shipped binary agrees with the whole-workspace build on all of them. `doc/verify.md` has the
commands, because that is a claim that decays.

**And session 660's report is real, with its attribution corrected.** It said
`cargo nextest run -p pdf-model` alone fails six CCITT tests where `--workspace` passes, black and
white exchanged, and blamed `hayro-ccitt` feature resolution. `hayro-ccitt` at the pinned revision
**has no `[features]` section at all**; `CCITTFaxDecode` goes through the worker; and "black and
white exchanged" is trap 10's own sentence from the seventh session.

#### And the census can say so now, which is one of the two residues

The seven-hundredth session left "the census counts no reports" as owed, and it is paid (ADR 0573).
`MarkedSpan::enclosed_a_refusal` attributes an `Unsupported` to the marked-content sequence that
enclosed it, `AccessibilityNode::enclosed_a_refusal` carries it to the boundary, and the census
prints how many elements have **both** no place and a refusal inside them, per page, with the
page's own report sentence. Run against the defect above it names the nine exactly — three pages of
`issue5481.pdf`, three elements each, under `an image (Im0: starting the sandbox worker failed …)
was not drawn` — and accounts for the whole of the −9 in *placed by their own marks*.

**Trap 11 decided the condition and is the part to copy.** The reflex condition is *placeless on a
page that reported*, which fires on every placeless element of a page whose report is about
something else on the sheet; §14.8.3.3 states **enclosure**, so the refusal is attributed to the
sequences the element encloses and to no others. And the class claims enclosure rather than
*cause*: `issue8702.pdf`'s two elements enclose a refusal and still have a place, because they drew
text as well.

`text_extraction` never had this gap — it prints `{incomplete} incomplete and not gated` and
excludes a reporting page by name. `selection_census` does not count reports either, and its shape
means a refusal arrives as a *missed drag* rather than as a smaller number, which is the loud
direction; attributing a miss to a report is owed and named in ADR 0573.

### 10a. A cached reference render is a fourth thing that can be stale

The key is built from the invocation itself plus the renderer's version and the document's
SHA-256, so **a flag not in the key is a flag not passed to the renderer either**. What it cannot
see is a renderer whose output changes while its version string does not. **The variable names a
*directory* and only the literal `off` disables it** — `PDFREF_CACHE=on` silently starts a fresh
319 MB cache in a directory called `on`. **The hit rate is printed and it is the tell**: under 99%
on an unchanged tree means the corpus or a renderer moved. A remembered *timeout* is the one entry
whose truth decays, counted separately and expiring after a week.

**And a remembered *failure* carries the wording of the run that stored it.** The key is the format
tag, the renderer's version, the document's digest, the page, the resolution and the invocation —
and not the harness's own prose, so a round that improves a failure message leaves every stored
`.err` entry saying what the previous version said. It is a message rather than a verdict, and
bumping `FORMAT` to flush it would invalidate tens of thousands of stored *renders* to correct a
few dozen sentences, so the fix is to delete the `.err` entries and let them re-derive — one
renderer invocation apiece. Same family as the paragraph above: a cache's key is a claim about what
makes two answers the same answer, and prose was never in it (ADR 0574).

**And a stored entry can be *incomplete* rather than stale, which is what a `FORMAT` bump is for.**
The renderer's own log sat beside its image in the work directory and was not in the entry at all,
so a hit restored a picture and no words — invisible while the log was only a diagnostic, and a
defect the moment ADR 0769 made it part of a verdict, because a rule reading a file that only a
*miss* writes reaches one verdict on the first run of a corpus and another on the second. The
paragraph above rejects a bump to correct a few dozen sentences and that stands; this was a change
to what an entry *means*, which is the case `FORMAT` exists for. It cost one re-render of all 6707
entries, about 1300 seconds of processor time, with every verdict in every class unchanged — and
that run is the control, not a formality: it is the only thing that says the bump moved nothing but
the cache.

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

**And a sixth instance, in a *count* rather than a report, where the condition was source text.**
`tools/state.sh windows` asks which of the boundary's questions each window reaches, by looking for
`Query::X` in the host's crate — and its first run said both native hosts reached §12.3.5's
collection. They do not. The evidence was one line of `viewer-host/src/panel.rs` reading *"a
different answer ([`viewer_core::Query::Collection`]) that **this host does not yet ask**"*, so the
count reported the opposite of what the sentence said, four words later. **A count over source text
is a claim about what the text *is*, and a comment is text**; `state.sh hosts` had carried the same
condition since ADR 0509 and had simply not been bitten. Both strip `//` to end of line now. The
general form is the one that generalises past greps: whenever a condition is *presence of a name*,
ask what else in the population can carry that name without meaning it (ADR 0577).

**And a seventh and an eighth, in the same count, twelve rounds later — because that question was
asked once and not exhaustively.** Stripping comments answered *what else can carry the name*, and
two more things could.

**A suffix is not a name.** `grep -oE "Command::[A-Za-z]+"` matches the tail of
`PathCommand::Close`, `pdf_render`'s *path* close, which `viewer-ui` writes on every rounded
rectangle of its own chrome — so "does this window ever close a document?" was answered by a piece
of chrome geometry. `\b` is the fix and the rule is one word longer than the sixth instance's: a
grep for an enumeration's variant is a claim about a **path** through the source, and a suffix is
not a path.

**A rule written in the paragraph above the code is not a rule the code has.** `section_hosts`'
comment says, in so many words, that it asks `viewer-ffi` alone because *"`viewer-ui` names all of
them in `trace.rs` … so the same grep over those two would answer 100% and mean nothing — trap 11's
shape, a count whose condition is not the question"*. `section_windows` was then written sixty lines
below it, over a population containing `viewer-ui`, citing that same comment as its reason for
excluding `viewer-confined` and not excluding the trace formatter. A match arm that formats a
variant's name is a name **printed**, not a question asked, and the count said `25 of 25` where the
host sends 22.

**What the two of them cost is the sentence worth carrying**: the section's headline finding —
`every Command reaches at least one window` — was false, and the two messages it was wrong about
were `Close` and `Focus`. **A count you wrote is a population you have not audited**, and the audit
is not "is the condition right?" but "list everything in this population that satisfies the
condition and does not satisfy the question". ADR 0603.

**And a ninth, about an exemption's *other* direction: a set-aside that takes a measure rather than
a document is invisible to every denominator the gate prints.** The word-box gate stopped applying
its cross-axis bound where no font on the page states §9.8.1 Table 120's pair, and the round
calibrated that both ways (trap 13). Disabling the set-aside failed loudly. **Widening it to
everything failed nothing at all**: the judged set, the pair count, the refusal table, the verdict
and the named list of out-of-bounds documents were all identical, because the set-aside costs a
*measure* of a word that stays in every count — and the one document whose words failed that bound
failed the other one as well, so no name moved. A printed count of what was set aside is what
ADR 0756 added and it is not enough on its own; what closes it is a floor under the population the
bound is still applied to, ratcheted like the judged set. **Ask of any exemption: if it grew to
cover everything, which assertion would fail?** If the answer is none, the exemption has no floor
under it yet. ADR 0759.

### 13. A sweep for a defect must be run against the defect before it is believed — and trap 29, the lifting that is the same experiment with the sign reversed

**Trap 29 is the same rule applied to a *constant* and is the last section of this trap**
(`doc/reviews/1018-what-retiring-a-trap-would-cost.md`): a lifting experiment is a sweep whose
instrument is a number, and its calibration is this one's with the sign reversed. Both incidents
are kept and both numbers stay citable (ADR 0232 §2).

#### 13. A sweep for a defect must be run against the defect before it is believed

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

#### 29. A bound lifted in a scratch build is lifted only where the code reads the constant

"Lift the bound sixteenfold and see which documents still reach it" is this project's standard
experiment on a budget, and ADR 0271 ran it on `MAX_FORM_DEPTH` over 65 944 documents: all four
witnesses reached 256, so all four were cycles, so the bound was the attack it exists for. Two more
rounds repeated it over two more corpora and the eleven-document claim went into the constant's own
comment. Twenty-five of the twenty-seven were finite — a tiling cell was run at `MAX_FORM_DEPTH - 1`,
so lifting the constant lifted the cell's starting point with it, and a cell holding two levels of
forms reported the bound at sixteen, at 256, and at any value a scratch build could name (ADR
0793).

The experiment had no control. The rule above — run a sweep against the defect before believing
it — has a mirror for a lifting: **run it against a document known to be finite and
deep, which must stop**, before believing that whatever still reaches the lifted bound is a cycle.
And read every site that *derives* a number from the constant, because those move with it: a
`grep` for the name finds them in a second and the eight-hundred-and-seventy-first session, which
found the two nestings, did not look.

The defect is planted in every spelling it is actually written in. CLAUDE.md's grep for history in
code comments (`hundred-and-|session`) came back with nothing but false positives while 55 comment
sites in 20 files said `round 911` and the like (round 1423, ADR 1680): the sweep had been checked
against the spellings its author imagined, not against the tree's. Before a sweep's clean answer is
believed, one real instance of the debt is found by eye and the sweep is shown to name it.

A census that `continue`s past a document it cannot open has silently shrunk its population: round
1440's Lab census skipped every encrypted file without a word, and the arms then moved a page it had
never seen (`bug1782186.pdf`, a D65 Lab under a `Separation` in an encrypted file). A census opens
encrypted documents with the corpus passwords and prints every document it could not open, so a
skipped file is a line in the output rather than a hole in the count.

A figure parsed out of prose a person writes is a sweep of the same kind. `tools/state.sh batches`
read each commit body's round durations with a regex that took one digit group, and from batch 64 the
bodies wrote "7 497 s", so it printed 2 135 s for a 47 135 s batch and found no gate line, unnoticed
for eight batches (round 1456). A parser over prose is calibrated on one body read by hand every
time it is used, because the writer's format drifts and the regex does not say so.

### 18. A limit a process is under can destroy the channel it reports through

`viewer-confined`'s worker inherited the host's standard error, with a comment saying why: "so that
a worker that dies says so where the operator can see it". Its confinement sets `RLIMIT_FSIZE` to 0.
Those two sentences are fine apart and contradictory together — **a limit on writing to files
applies to whatever the inherited descriptor happens to point at**, and on any host that logs to a
file that is the worker's own diagnostics.

What it looked like: the same document, the same worker, the same defect.

| the host's standard error is | the host is told | the worker said |
|---|---|---|
| a **pipe** | `killed by signal 6` | `memory allocation of 1899996152 bytes failed` |
| a **file** | `killed by signal 25` | nothing |

Signal 25 is `SIGXFSZ`, so the report names a *file-size* failure for an out-of-memory abort — and
the round measuring it first read that as a defect in the code it was measuring. Both arms had to be
run before the pattern was visible at all, and the two arms differ only in how the *measuring
harness* was invoked (ADR 0597).

Two things generalise, and the second is the one worth carrying:

- **A pipe is not a file**, so `RLIMIT_FSIZE` does not reach it. That is the fix here and it is
  cheap: pipe the channel, drain it, echo it onward.
- **Whenever a process is put under a limit, ask what the limit does to the channel that reports
  the limit being hit.** `RLIMIT_NOFILE` and a seccomp filter have the same shape — a diagnostic
  path that needs a descriptor or a system call the confinement took away is a diagnosis that
  arrives as silence, and silence is read as a different failure. `tests/confined.rs`'s
  `a_confined_worker_cannot_write_a_diagnostic_to_a_file` pins this one on a single write.

### 23. `--all` and `--workspace` are scoped to a workspace, not to the tree

`cargo fmt --all --check` has never read a line of `fuzz/fuzz_targets/`. It does not say so. It
exits 0. Two rustfmt diffs sat there under a green formatting gate until the
eight-hundred-and-seventh session went looking (ADR 0739).

The mechanism is one sentence of cargo's: **a manifest with a `[workspace]` table of its own is a
workspace root, not a member of the one above it**, and `--all` and `--workspace` mean *every
package in **this** workspace*. `fuzz/Cargo.toml` declares one deliberately, so that `cargo-fuzz`'s
sanitiser and profile settings stay off the tree — a good decision whose consequence is that every
workspace-scoped command owes a second invocation naming that manifest.

§2 had learned this once already, for compiling, after fourteen rounds in which the fuzz targets did
not build against the tree they fuzz. It did not generalise the lesson to the command one line
above, because **a gate that is silent looks exactly like a gate that is clean** — which is this
group's whole subject and is why the trap is here rather than beside the build ones.

Three things to carry:

- **A workspace-scoped flag is a claim about the manifest graph, not about the directory tree.**
  Before believing a `--workspace` or `--all` run covered something, ask which workspace the file is
  in. `cargo locate-project --workspace --manifest-path <manifest>` answers it in milliseconds.
- **The module graph is the other way a file escapes a tool.** rustfmt walks `mod` declarations from
  each target's root, so a `.rs` file nothing declares is unformatted however the workspaces are
  arranged. This tree has none, and what says so is a measurement rather than an argument:
  `cargo fmt --all -- --emit stdout` prints every file rustfmt formats, and the set difference
  against `git ls-files '*.rs'` is the answer.
- **`tools/conformance/tests/workspaces.rs` holds the workspace half closed**, by deriving the
  population from cargo rather than listing it — so the next crate kept out of the workspace fails
  on the day it is added. It does not hold the module-graph half closed, and nothing does.

**And a lint level is a second thing that stops at the boundary, by a different road.** `[lints]
workspace = true` resolves against *this* workspace, and cargo offers no way to point one
workspace's packages at another's table — so a second workspace either restates the levels or has
none. `fuzz/` had none until the eight-hundred-and-tenth session, and `clippy` had therefore never
judged a fuzz target at all: thirty-three findings, five of them arithmetic in a target's own
counters under a profile that keeps overflow checks on, which is an abort waiting to be filed as a
crash in the parser under test. The gate now demands a `cargo clippy` line per root **and** compares
the tables, because closing one without the other closes nothing (ADR 0742).

### 24. A fuzz target's exit status answers *did it crash*, never *did it run*

The eight-hundredth session watched `page` execute 86 912 iterations at `cov: 0 ft: 0 corp: 0` and
exit 0. The eight-hundred-and-tenth measured what that is worth: from an empty corpus `page` reaches
**cov 103, ft 182 — the same two figures `document` reaches, to the unit**, because the two targets
share a prefix and diverge only once a document parses well enough to have a page tree. Identical
figures mean `page` never entered `pdf_model::interpret`, which is the entire reason ADR 0264 built
it. With this disk's corpus the same target reaches cov 37 083 and ft 169 360.

**No amount of wall clock closes that gap**, and that is the part worth carrying: a fuzzer will not
invent a header, a cross-reference section, a page tree and a resource dictionary that agree with
each other. A target over documents whose corpus is not documents is a target testing the recovery
scanner.

Four things to carry:

- **Read libFuzzer's own last line, not the exit status.** `cov:` and `ft:` are what say whether
  anything was exercised. `tools/fuzz.sh` does both halves — it refuses to start a target whose
  corpus directory is empty, and it fails a run whose final `ft` is zero — and it takes the
  invocation out of `doc/verify.md` so that the two cannot drift. **`ft: 0` is a fork-mode figure**:
  an ordinary run always reaches the harness on the empty input, so a zero there is a *parent*
  reporting an empty shared corpus, which is what round 800 was looking at and why `page` is the
  target it happens to.
- **And read its *first* line beside its last, because the last one cannot tell what the run did
  from what the corpus already had.** `INITED` is printed once the seeds are loaded and before a
  single mutation, so `INITED → DONE` is what the run length bought. Run at `doc/verify.md`'s own
  lengths, **eight of the fifteen targets add fewer than a hundred features and `forms_data` adds
  none at all** — cov 488, ft 1375 at both ends, measured twice. That is a *saturated* target rather
  than a broken one, and the consequence is how a round chooses work: against a mature corpus, more
  iterations find nothing and **seeds** are what move a target. `tools/fuzz.sh` prints the pair; a
  fork-mode parent has no `INITED` and it says so rather than subtracting against nothing (ADR 0747).
- **`fuzz/corpus` and `fuzz/artifacts` are gitignored, so "is this target seeded" is a fact about
  the disk and not about the repository.** No gate can read it out of the tree. A fresh worktree had
  neither directory at all until `tools/worktree.sh` was taught to link them, so *every* fuzz run a
  parallel round made started from nothing and said nothing about it — which is trap 23's sentence
  with a different instrument in it.
- **The general shape is wider than fuzzing.** An instrument whose input is gitignored can be empty
  without the instrument saying so, and it will still exit 0. Ask what the instrument read, and how
  much of it, before believing what it reported.

### 25. A hand-written population can name a thing that never existed, and finding nothing there reads as a pass

`tools/round.sh`'s build-script check asked two crates by name, `pdf-font` and `conformance`.
**`tools/conformance` has never had a build script — not in any commit of this repository** — so
half of every run since the check was written looked for a thing that does not exist, found
nothing, and printed a `✓`. Meanwhile `crates/pdf-sandbox/build.rs` bakes its manifest path with
`env!` and then *reads a directory under it*, which is precisely the failure the check exists to
predict, and it was never asked. The build directory on this machine holds a `pdf-sandbox` build
script naming a checkout that is gone; the old check could not see it and the new one counts it
(ADR 0752).

Two more instruments in the same two files had the same shape. `tools/worktree.sh list` globbed
`pdfv-r*` — the names it makes itself — under a heading about *orphaned* build directories, so it
could only ever report its own; the directories in the root it could not name were most of that
root's size. And `tools/state.sh disk` reported the round's own
`target-dir`, which is right for trap 15's reason and is two orders of magnitude away from the
figure `doc/todo/02` §5a's hundred-gigabyte threshold is about.

**The failure is not that the population was wrong. It is that a narrow population and a clean
tree produce the same output** — which is trap 23's sentence with the instrument's *input* wrong
instead of its scope, and trap 24's with a list instead of a corpus. So:

- **A population written by hand is a claim about the tree, and it decays in both directions**:
  it names things that have gone or never were, and it misses things that arrived. This tree
  already knew that — `tools/worktree.sh`'s gitlink guard derives its paths from the index for
  exactly this reason, and its own comment says a hand-written list "goes stale the next time
  something is linked, which is exactly how this one did". The lesson had not been carried two
  functions down the same file.
- **Derive the population, and make an empty one *fail*.** The check above now reads every
  tracked `build.rs`, keeps the ones using `env!("CARGO_MANIFEST_DIR")`, and reports rather than
  passes when that comes back empty. A check with nothing to ask must not be silent.
- **The discriminator has to come off the source, not off the artefact.** `crates/pdf-spec` reads
  the same variable through `std::env::var_os`, so cargo supplies the live value and it cannot go
  stale — yet its compiled build script carries the path in its debug info all the same. Grepping
  a binary for a path finds strings the program will never read; what the *source* does with the
  variable is the fact.
- **A line range is the same trap at its smallest.** The usage text in `tools/worktree.sh` was
  `sed -n '3,20p'`, four lines past the header block it meant, printing `set -euo pipefail` at a
  reader. Every edit above a hard-coded range invalidates it, and nothing says so.

**And it has a mirror image, which is worse because the sweep looks busy.** A hand-written
population can also *omit* something that does exist, and a page it never named reads exactly like a
page that was clean. `doc/todo/00`'s step 7 had been a recipe since the two-hundred-and-sixty-fifth
session, rebuilt by hand at least fifteen times; the nine-hundred-and-seventy-fourth session's
rebuild measured 775 of the 838 pages it listed, and the gate had printed 839. The 63 it silently
lost were the whole `doc/corpora/pdfbox` population — the gate *prints* a page as
`pdfbox/cweb.pdf page 10` and *writes* it to `pdfbox/cweb/p10/…`, so the corpus label is a
directory in the path and not part of the file's name. The head, the alarm count and every printed
row looked exactly as they always had, and five rounds of that sweep's "null" were right answers
over an unstated denominator.

The defence is the same one in both directions and it is one line: **print the denominator, and
make a member the sweep cannot measure a non-zero exit rather than a row it quietly drops.** That
is what `cargo run --release -p pdfref --bin undrawn` does, reading its population *and* its
exclusions off the gate's own report rather than assembling either (ADR 0985).

A brief's "the N lines" is one of these. Batch sixty-five's instruments slot was told the tree held
three bare-`flock` instructions in two files; a sweep over the tree found twelve in eight (round
1419, ADR 1674). A sweep a brief asks for is widened from the lines the brief names to the tree,
and the test that holds the result derives its population from the tree too.

A brief's list of API members is the same thing. Batch sixty-nine named six `ANFB_*` filters for the
script round to build; Adobe's reference at the pinned commit lists six beside `ShouldNone`, and two of
the brief's six (`ShouldNoView`, `ShouldNoZoom`) are not among them (round 1438). A member list in a
brief is read off the source at its pinned commit before anything is built from it.

And a population counted by the wrong part. A brief sized a build on "thirteen signatures" under one
policy; the build needed the URL qualifier, which three of the thirteen carry (round 1446, ADR 1728).
A population is counted by the part the build actually reads before the build is sized.

### 27. An assertion on a substring passes for every answer that shares it

Trap 11 on the other side of the wire. A report is only as good as the condition it fires on; an
**assertion** is only as good as what its expected value *excludes*.

`pdf-syntax/tests/encryption.rs::an_unspecified_revision_is_refused_by_name` opened
`issue21579.pdf` and asserted the refusal's sentence contained `"/R 5"`. Delete §7.6.4.2's refusal
entirely — accept revision 5, which Table 21 says "[s]hall not be used" — and the same document is
declined a few lines later by §7.6.4.1's crypt-filter pairing, whose sentence begins *"/R 5 with a
crypt filter method"*. The substring is there, the test is green, and the clause the test is named
after is not implemented at all. Two ledger rows rested on it.

**Revision 5 is implemented now** (ADR 0820), so the counterfactual above — *accept revision 5 and
watch the test stay green* — is the tree's actual behaviour, and the test that survives asserts on
`/R 7`.

The tell is that the expected value was assembled from the *input* rather than from the answer: a
document stating `/R 5` will have `/R 5` in almost any sentence about it. So:

- **Assert on what distinguishes the answer from its neighbours**, which for a refusal means the
  clause's own words — and check that the neighbours exist: the question to ask of any error
  assertion is *what else can this input produce here, and would my assertion accept it?*
- **A test named after a clause is a claim about that clause**, so the plant that calibrates it is
  the removal of that clause's code and nothing else. A plant somewhere adjacent failing the test
  is not evidence about it.
- The same shape is why trap 11's sixth instance is a count over source text: *presence of a name*
  is a weak predicate wherever something else in the population can carry that name without
  meaning it.

### 30. A sink keyed by name hands its outputs back in the order they were *opened*

`pdf_transform::MemorySinks` says so in its own doc comment — "in the order the outputs were
opened" — and `split` opens them from inside a `rayon` map, so the order is whichever thread got
there first. Three of `tests/split.rs`'s assertions indexed the returned vector by position
anyway, and for two rounds every run happened to schedule the pieces in order. The
eight-hundred-and-eighty-eighth session's first run of a *different* test scheduled them the other
way and `assert_eq!(Pages::new(&first).len(), 2)` failed with `left: 1, right: 2` — a gate
reporting on the scheduler under a name that promised something about the writer.

**The rule is about what the collection is keyed by, not about parallelism.** A sink hands back
`(name, bytes)`; the name is the identity the pattern gave the output and the position is an
implementation detail of the collection. So a test looks an output up **by its name**, and a
helper that panics naming every output it did have is what makes the failure legible when the name
is wrong. The same applies to any report, listing or map whose producer is parallel: `Report`'s
`outputs` *are* in plan order because the verb assembles them that way after the join, and the
difference between the two vectors is exactly the kind of thing no type says out loud.

### 31. A fallible filesystem call is not a *safe* filesystem call inside the confinement

`pdf_font::substitute` walks `/usr/share/fonts` to stand in for a font a document names and does
not embed, and it is written to shrug the walk off: `let Ok(entries) = std::fs::read_dir(dir) else
{ return; };`, and `find` then answers from the compiled-in faces, because a machine with no fonts
installed is a supported deployment. Every line of that is correct and none of it runs inside
`Profile::Interpreter`. **`SECCOMP_RET_KILL_PROCESS` does not return an `Err`** — the `openat`
ends the process, the `else` branch is never reached, and the mount loses the whole generation
(the viewer, the page). Four of the first sixty documents the read side's corpus walk touched did
this, each of them naming a CJK or Arabic face; ADR 0870.

So the population to look at when a crate is linked into a confined worker is not "code that
unwraps" or "code that can fail" — it is **code that opens**, however carefully it handles the
failure. And the fix is never to widen the filter: a process that needs something off the disk is
told, before the confinement, that the disk is not there, so that it takes the path it already has
for a machine that does not have the thing. `pdf_vfs::confine` and `viewer_confined::confine` are
where such a statement goes; both already ask `available_parallelism` and `address_space_in_use`
there, each under a comment saying this is the last moment it can be asked.

**And the probe for it is cheap, which is why there is no excuse.** A test that re-executes itself,
confines itself exactly as the worker does, and then does the suspect thing costs one exit code and
is calibratable against the tree without the fix (trap 13) — `unix_wait_status(159)` is signal 31,
`SIGSYS`, and is what "it was killed" looks like from the parent.

### 32. A confined worker cannot **drop** an owned descriptor, and only the debug build dies of it

Trap 31's sibling, and the one that is invisible in the build a person runs. ADR 0812 hands a
confined worker a descriptor with `SCM_RIGHTS`; `doc/todo/59`'s resource port wanted to hand it a
second one. Receiving works, `pread64` works — and then the `std::os::fd::OwnedFd` is **dropped**,
and `OwnedFd::drop` asks `fcntl(fd, F_GETFD)` before `close` to catch a double close, under
`core::ub_checks::check_library_ub()`. `fcntl` is not on the allow-list, so the worker is killed by
`SIGSYS` on the line *after* the read succeeded:

```text
pread64(3, "OTTO\0\f\0\200\0\3\0@CFF …", 82264, 0) = 82264
fcntl(3, F_GETFD)                       = 0x48
+++ killed by SIGSYS (core dumped) +++
```

Two things make this worse than trap 31 rather than a repeat of it. **The check is compiled per
build**, so a `--release` worker survives and every debug worker dies — and every gate in this tree
runs debug binaries, so the shape of the failure is "the measurement I ran by hand works and the
gate is red". And **no filesystem call appears anywhere in the code**: the population trap 31 names
— "code that opens" — does not contain it. The population here is *code that owns a descriptor and
lets it go*.

There is no way to close one from safe Rust that avoids the check, because every std type that owns
a descriptor closes through `OwnedFd`. So the first answer is not to hold a descriptor a confined
worker will ever drop: ADR 0880's port sends the resource's **bytes** on the frame instead, which is
what a *face* does.

**The document's descriptor cannot take that road, and session 924 is where the second answer was
spent** (ADR 0888). ADR 0812 hands it over precisely so that a 6 GB file does not cross as bytes, so
`fcntl` is admitted for the interpreter profile alone, **narrowed by argument** to `F_GETFD` and a
kill for every other command. `doc/todo/61`'s table now has two shapes in it and says which answer
belongs to which; the order still matters, and "can the resource cross another way?" is asked first.

**And the trap has a second half, about the test rather than the defect** (ADR 0889). Because the
check is compiled per build, a probe written the natural way — hand the process a descriptor, drop
it, see whether it survived — **issues no system call at all** under `--profile gates`, which
inherits `release`. It would be green on the day the rule was reverted, in the profile most of
`doc/todo/02` §2 runs under. So a probe for a behaviour that exists only in some build profiles
**issues the call it is about, by name** — `rustix::io::fcntl_getfd` rather than a `drop` — and the
end-to-end witness is kept beside it while being honest about the one run where it binds
(`cargo nextest run --workspace`, whose worker is a `dev` build). It is ADR 0498's lesson from the
other side: there a gate turned a shipped *setting* off, here a gate profile turns a *check* off.

### 33. A counter of what was *produced* cannot see a cost paid in *validation*

`pdf_vfs::Vfs::generated` counts how many virtual files the tree has produced the bytes of. It is
the right instrument for the thing it was built for — ADR 0865 §3's size notes, whose whole claim
is that a second `stat` produces nothing — and `tests/read_corpus.rs` and `tests/a_write.rs` both
assert on it.

It read **1** while a mount spent 176 ms on each of twenty thousand questions about one directory,
because the extraction was being re-run to check that a *name* existed rather than to make bytes
(ADR 0886). The bytes were produced once, cached, and served from the cache every time; the
counter's own sentence stayed true throughout, and the defect was a hundredfold. Two corpus walks,
a whole gate sequence and a `regenerated` report all looked straight at it and saw nothing; what
saw it was a wall clock in an example.

**So a property about how often something runs has to count that thing running.** The general
shape: a counter names an *event*, and a cost is only visible to it where the cost is that event.
Work done to answer a question — resolving a path, validating a name, deciding a refusal — is
invisible to every counter of outputs, and it is exactly where an accidental quadratic hides,
because it looks like nothing at all: no allocation, no output, no error. Where the property is
"the expensive call happens once", wrap the expensive call and count *it* — a counting decorator
over the worker, the generator, the trait, whichever thing you mean — rather than asserting on a
count that is nearby and cheaper to reach.

It is trap 25's shape one level in: there the population was wrong, here the *event* is, and both
fail by returning a clean answer to a question nobody asked.

### 39. A signal that always fires has stopped being a signal, and it looks like caution

`tools/round.sh` printed whether this was a **fifth round** — the round that ran `doc/todo/02`'s
whole gate sequence whatever it touched, because a change→gate map is a claim about the crate graph
and a claim decays (the merge runs that sequence now, ADR 1638). It computed that from the session number, and it took the session number from
the branch name, which on a campaign branch is the number the *campaign* started at. On
`round-945/the-fifth-round` it printed `session 945` for fourteen rounds, and because 945 divides by
five it printed **a fifth round** for all fourteen (ADR 0969).

Nothing broke. The full sequence ran where the core four were owed, which costs minutes and hides
nothing, and that is exactly why nobody looked: **the failure was in the safe direction, so it read
as caution rather than as a defect.** It was found by a round writing the `doc/history/` files it
owed, which is the only job that reads the session number and the directory it is derived from in
the same minute.

What was actually lost is the distinction the signal exists to make. A periodic unconditional check
and a targeted one are different claims about a round, and fourteen rounds now have no record of
which they were — including the three that genuinely were fifth rounds.

This is trap 11's other failure mode and it is the harder one to see. **Trap 11 is a report that
fires on the wrong condition; this is a report whose condition is a constant.** A gate that always
passes is the shape everybody watches for. A gate that always fires is the same defect wearing the
reassuring half, and the only way to catch it is to ask an instrument, once, *has this ever answered
the other way* — which for a periodic signal means checking that the period is real and not that the
signal is on.

The fix generalises past this script. The wrong number came from a **hard-coded preference between
two sources of a fact**, each of which goes stale in its own direction; the comment defending it
said the branch name "cannot go stale". Where two sources are both floors, take the later of the two
and print which one won — then nobody has to remember which to trust, and a workflow that has
outgrown one of them is visible in the place every round already looks.

### 34. A guard has to be made of the same stuff as the figure it guards

`crates/viewer-ui/tests/launch_path.rs` measures principle 2's four numbers, and every clock figure
it judges is **one first pass in a fresh process** — an open, a first page, a page turn, done once,
by a process that has not done it before. The guard that decides whether the machine was the
machine the bands were taken on is a fixed piece of the same work run **fifty times inside one
process, after that process has already run its phase, of which the quickest is kept**.

Those are two different measurements. By pass fifty the allocator, the caches, the branch
predictors and the core's own clock have been warmed by the forty-nine before it; a first pass has
none of that, and neither does any figure the gate judges. Session 931 measured the gap over
twenty-six consecutive runs on `main`: the probe moved **1.3 %** — 0.703 to 0.749 ms — while a
five-page document's *warm* open read 0.45 ms in nine runs and **1.01** in another, with the probe
at 0.705 in both. The same fixed work measured as one first pass reads about **1.6 times** the
fifty-pass minimum, and it is that ratio, not either number, that a contended machine moves.

ADR 0884's own sentence — *a guard has to sense every subsystem the figure it guards is made of* —
is the version of this about **which** subsystems, and the gate satisfied it: there is a processor
probe and a disk probe. This is the version about the **state** those subsystems are in, and it
does not follow from the first: a probe can touch every subsystem a figure touches and still
measure all of them warm.

**So when a probe and a figure disagree, ask how each was taken before asking what moved.** A
minimum over repetitions, a mean, a warm cache, a second run of the same loop — each is a filter,
and a filter that removes exactly the thing the figure is exposed to makes a guard that cannot
fail. The cheap check is to measure the *same work both ways in the same process* and print both:
where the two track each other the probe is sound, and where they do not the difference is the
guard's blind spot, in the units the figure is in.

It is trap 33's shape in the other dimension. There a counter named the wrong **event**; here a
probe names the right work in the wrong **state**, and both come back with a clean number about a
question nobody asked.

**And there is a third dimension, which is the units — the sharpest of the three, because nothing
in the gate's shape hints at it.** The same gate's `peak_mib` is a *memory* high-water, classified
as needing the guard, and the guard is a **clock**. Session 934 ran the sequence on a machine with
9 GiB free after two neighbours' corpus walks: the calibration probe read 0.706 ms inside
`0.62 .. 0.78`, so the run judged, and all four documents' memory high-waters came in a quarter
below their minima together — 99, 103, 104 and 116 MiB against floors of 127, 131, 132 and 143. Nine
re-runs on the same binary with 29 GiB free read 161 to 182 MiB, inside every band, two of them
judging every figure with nothing outside. `open_peak_mib`, the memory figure with **no graphics
device in it**, did not move by a megabyte in either state, which names the mechanism as the
driver — the resident pages of its shared objects, which the kernel reclaims under exactly that
pressure and nothing in the process can see (trap 35, ADR 0910). A clock cannot sense an allocator, and would read in band on a machine with a gigabyte free
and on one with sixty. So: *the same work, in the same state, and in the same units* — and the third
of those is the one a `steady: false` classification can look like it has already handled. ADR
0909.

### 35. A process's resident high-water is mostly its libraries, and the kernel decides how much of them is resident

`VmHWM` — and `getrusage`'s `ru_maxrss`, and everything else that says *peak resident* — counts
**every** resident page, and pages of a mapped file are most of them as soon as a process links
something large. The launch gate banded that figure as "the memory high-water" for thirteen
rounds; measured, a process that brings this machine's graphics device up and does nothing else has
a 108 MiB high-water of which **97 MiB is file-backed** — 52 of them `libLLVM.so`, which is 163 MiB
on disk — and 11 MiB is memory the program asked for.

The half that is not the program's does not hold still, and not for any reason a program can see: a
page fault on a mapping whose pages are already in the page cache maps a whole fault-around window,
and a fault on one that has been evicted maps a single page. So **what a neighbouring process read
an hour ago moves this number**. Measured by evicting two libraries and changing nothing else, the
same binary's high-water fell 25% while its anonymous total moved by 30 KiB; over one afternoon the
same figure was seen at 92 MiB and at 180 MiB.

Five rounds met that as a regression, four of them correctly refused to widen the band, and none
could decline the figure either — because there is no clock in it, and none of the gate's probes
senses the page cache. ADRs 0910 and 0911.

**So a memory gate bands `Rss - file-backed`**, which off `/proc/self/smaps_rollup` is `Anonymous`,
and prints the whole-process figure beside it unbanded. Two things follow that are worth having in
hand before the next one is written. A resident figure taken in a process with a big dependency
graph is a **measurement of the machine's page cache** to within a factor of two, whatever it is
called. And the anonymous figure has a granularity of its own where transparent huge pages are
`always`: a 2 MiB huge page is 2 MiB of `Anonymous` however much of it is touched, so a band under
about ten mebibytes needs one huge page's headroom or it will fire on a step the program did not
take.

### 37. A digest of the artefact cannot see a change in the *diagnosis*

`examples/display_list_digest` is what a round runs to claim it drew nothing differently, and its
own doc comment says so: "run it on two revisions and `diff` the two files; an empty diff is the
claim". For most of its life it hashed the display list and nothing else.

The nine-hundred-and-thirty-sixth session changed how a dimension written as a real is read, and
the whole of what moved was a **sentence**. `GHOSTSCRIPT-695872-0.pdf` states
`/W 737.999999999715 /H 49.999999999` over a JPEG whose own frame is 738 × 50; §7.4.8 puts the
dimensions in the encoded data, so the samples were always drawn on the codestream's grid and the
dictionary's numbers never reached a command. What the dictionary decided was whether this reader
*accused the file* of a disagreement — and under truncation it did, about a contradiction that was
its own arithmetic. Before and after: 673 commands, identical bytes, identical hash, **one report
against zero**.

So the digest would have printed an empty diff for a change that fixed a false accusation, and a
round would have read that as "nothing moved" and been right about the wrong artefact. It hashes
`interpretation.unsupported` beside the list now.

The general shape, which is worth more than the instrument: **an interpretation is two things, what
the program drew and what it said, and an instrument that captures one of them answers "did
anything change" with half a fact.** It is trap 11 from the other end — that trap is about a report
firing on the wrong condition, this one is about nothing being able to *see* that it did — and it
is trap 33's shape again, a clean number about a question nobody asked. When a change can move a
report, the diff has to be able to hold one.

### 41. A display list's golden digest is its `Debug`, so a new field on a vocabulary enum moves every page using that variant with no pixel moving

`raster_golden` holds two digests per page — the raster's and the display list's — and the second is
`format!("{:?}", list)`. When round 1190 gave `ShadingKind::Mesh` a field, ten rows moved: six were
type 6/7 shadings whose rasters changed, and four were type 4/5 meshes whose rasters did not — their
digest moved because the variant's `Debug` gained a field. "List only (an interpreter change no pixel
shows)" reads as a finding until the two populations are separated. Round 1160's overprint flag moved
966 rows the same way. The rule: when a round adds a field to a `pdf_render` vocabulary type, count the
digest-only rows as a population and show they are exactly the variant's users — interpret each and
count its commands, do not infer — before reading any of them as a change (ADR 1217).

### 42. A refusal's message is only as good as the branch that emits it

`redact.rs`'s module doc and ADR 1133 both said a `JPXDecode` image was refused with §7.4.9 NOTE 3's
reason — an over-budget decode is a reduced resolution level — and the match arm's `other =>` branch
had always given the generic "codec this removal does not re-encode" sentence instead (ADR 1248).
The same shape one level up: the ADR's sentence stated a genuine *condition* and the code refused the
whole *codec*, including every file the condition does not hold for. Trap 27 says an assertion is
only as good as what it excludes; when a doc comment or an ADR promises a specific reason, read the
branch that prints it, and when a refusal's written reason carries a qualifier, check the code tests
that qualifier.

### 49. A citation-spelling sweep changes what the quotation gate can attribute

Rewriting raster's `doc §N` into `doc section N` silently un-attributed sixteen blockquotes: the
gate had read the `§` as the attribution to a project document, fourteen quotations became
unattributed, and two fell through to an earlier ISO clause and failed as not verbatim (ADR 1314).
`citation::section_in_prose` now reads a project document named before "section N" as the
attribution. Run the quotation gate after every chunk of such a sweep, not at its end.

### 55. A note's figure can be made false by an ADR that never names the note

Sixty-nine oracle notes were flagged as overtaken; eighteen had a sentence made false and were
corrected, but seven carry a NUMBER that a coverage or construction ADR (1082, 1102, 1341, 0358)
changed without mentioning the page (round 1259, ADR 1355). Rewriting such a sentence carries the
old figure forward under a new citation. A figure is corrected only where an ADR's own table gives
the new one; otherwise it stays named in a re-measure population until a run re-takes it.

### 61. A path resolved by a sibling's prefix lands on the wrong rung

`pointers::Tree::holds` resolved `doc/corpora` as live because `doc/corpora-own` exists, so a skip-list
entry and a real file sharing a prefix silently decided which rung a pointer fell on (round 1271).
Match the whole path component.

### 63. A gate run with `--ignored` over a file with no ignored test checks nothing

`doc/todo/02`'s change-to-gate map named `-p pdf-transform --test gate` with no `--ignored`, and the
file's one test is ignored, so the line ran zero tests and exited 0; a `render-gpu` line had the
opposite shape (ADR 1392). `tools/conformance/tests/batch.rs` now matches every gate line's flag
against the file's own `#[ignore]` attributes, and `gates()` fails any line whose summary shows no
test passed. A green line is read for what it ran, not for its exit status.

### 68. A sweep that excuses a line for a phrase hides the history beside the phrase

The comment-history sweep's first version excused any line holding a legitimate use of "session",
which also excused the history sentence on the same line, and paired backticks across a joined line,
which swallowed "session 625" between two code spans (ADR 1403). Remove the excused phrase and read
what is left; pair code spans per line.

### 74. The overtaken sweep reads `ADR NNNN`, not `ADRs NNNN and MMMM`

A note citing two decisions in one phrase — "ADRs 0820 and 1377" — was reported as overtaken by
both, because the citation scanner matches the singular form only (round 1295). Cite each one as
`ADR 0820, ADR 1377`, and when a sweep names a note as citing nothing, read the note before believing
it.

### 75. A shared target directory hands a gate the other tree's `CARGO_MANIFEST_DIR`

Both the batch worktree and the main checkout built into one target directory; cargo names a
path package's artefacts relative to its workspace root, so an rlib built in main a few minutes
earlier counted as fresh for the worktree, and `strings` on the worktree's conformance test binary
showed `/home/cl/projects/pdf-viewer/tools/conformance` (round 1302). Every conformance run of that
batch had checked main's ledger and `doc/todo/65`, not the worktree's — trap 50 met by a gate rather
than a measurement. `tools/batch.sh open` now writes the worktree's own `.cargo/config.toml` (ADR
1440); until a batch has built there once, check the path before trusting a clean run.

### 82. A brief's clause or table number may be ISO 32000-1's

Round 1303's brief named §9.6.6.4, Table 122 and Table 111 for font substitution; in ISO 32000-2
those are nothing, the CIDFont descriptor additions, and the Type 3 operators. The sentences live at
§9.8.1, Table 120 and nowhere (the standard has no alias table for the standard 14). Grep the
heading in `doc/md/` before a number is cited or a sentence quoted under it, and name the
correction in the report.

### 91. A command that counts may not write

`tools/state.sh ledger` ran `--bin ledger`, which is the generator: it rebuilt the ledger from the
clause index and wrote the whole file back in canonical key order, so a row whose keys were out of
order moved on every run. Run in the owner's main checkout after a merge, it dirtied
`doc/conformance/ledger.toml` and the next fast-forward refused until the rewrite was discarded
(ADR 1487). The binary now counts unless passed `--write`; `main-checkout` runs git with
`--no-optional-locks`; every Python a section runs sets `PYTHONDONTWRITEBYTECODE=1`; and
`tests/read_only.rs` reads the script for redirections, writing commands, lock-taking git and
writing binaries. A section reads, and never writes.

### 95. A threshold and its quantity are read in the same space, and a dormant fallback is A/B'd

`separate_text` measured a glyph step in user-space x against a threshold in text space; under a
mirroring or scaling `Tm` the two disagreed. Measuring the step in text space (§9.4.4: the
displacement "shall be interpreted in text space") also made a long-dormant branch reachable — a
font with no space glyph had been given 0.25 × `Tfs` as its gap, exactly the producer's word gap,
so "Trace-basedJust-in-Time" had read as one word — and 1 205 of 2 809 corpus pages read back
differently, while the word-agreement gate, which folds whitespace, moved by nothing (ADR 1490).
A/B the readback over the corpus on one build behind a scratch switch before a gate is believed.

### 97. A table nobody gates is a table that drifts

`doc/performance.md`'s turn and step rows were measured by whichever round re-took them and held
by no gate; two merges doubled three of them — the photograph's turn from 32 to 76 ms when ADR 1495
declined every scan without an `EOI` (a defect: the restart plan never needed it), `issue14415`
by ADR 1443's two chords per piece (a cost chosen for §10.7.4's area rule, kept), the Type 3 page
by both — and nothing failed until round 1335 re-took the table a batch later; round 1339
bisected them by hunk across six exported commits. Principle 2 says a
page-turn regression fails the build; `turn_path` (ADR 1513) bands every row in
`doc/checks/turn-path.toml` the way `launch_path` bands the open. A row re-taken for a cost paid on
purpose carries its reason beside it; a slowdown nobody chose is a failing gate.

### 98. A ceiling that moves with no function of the tree moving is the binary's layout

Three rounds found the launch gate's `open_kinstructions` on `xfa_filled_imm1344e.pdf` at 1821–1823 k
against a ceiling of 1820 k, and each callgrind diff by function showed no function of this tree
moving: the whole difference was `pthread_getattr_np` — std's main-thread stack guard parsing
`/proc/self/maps` before `main` — reading 49 map lines where the base read 48. The band was raised
to 1825 k with that reason beside it (round 1336). A/B the change compiled out and diff by function
before a ceiling is moved or a change is blamed.

### 99. A copied binary looks for its worker beside itself, and a greeting bump needs a fresh one

A drive or census run from a copied binary finds no `pdf-sandbox-worker` next to it and panics;
`quorra-confined` after a `MAGIC` bump (`PDFVCF06` to `PDFVCF07` in one batch) needs
`viewer-confined --bins` rebuilt or the worker refuses the greeting (round 1333). And a source scan
that decides what kind of evidence a test is skips comment lines: `viewer_core::notes::about` names
`doc/pdf.js` in a comment, which classed every test calling it as a corpus witness (round 1337).

### 100. A check that fails only for tracked files passes in every round and fails on the merge

`crates/render-raster/tests/turn_path.rs` was untracked while round 1339 ran tier 1 and tracked
once the batch was committed; `state_sections` names every tracked ignored test file in
`doc/todo/02` §2 and prints the untracked ones, so it passed for every round and failed on the
merged main for a batch (round 1345). The main checkout builds in its own directory now, so the
conformance tests run there right after the fast-forward, before `install` and `close`.

### 107. A seeded fuzz corpus goes stale with nothing failing

`fuzz/corpus/<target>` is generated data linked from the main checkout, and nothing compares it with
what the seed scripts would write today. Round 1355 ran `forms_data` for twenty minutes from a disk
corpus that reached coverage 270; fresh seeds reached 1 241 — `display_list` likewise, 105 against
956 — and both campaigns were re-run. Before a campaign, run the target over the disk corpus and
over fresh seeds and compare libFuzzer's `INITED cov` — `fuzz/seeds.sh check <target>` does both
(ADR 1559); when they differ, re-seed in the main checkout first. And a bounded campaign's one crash
may be a known one, which is why a campaign meant to last runs `-fork=1 -ignore_crashes=1` as
`doc/verify.md` says; `jpeg_bands`' was `zune-jpeg`'s overflow (Q227), patched on the owner's fork
since A227.

### 109. The worker a gate spawns is as old as the last build, and a copied binary has none

A corpus gate waits behind the heavy-walk lock while five siblings edit the tree; when its turn
comes it spawns `pdf-sandbox-worker` from the last build, which no longer matches the library it
links against, and fails at once (round 1363's first corpus attempt). The walk builds `pdf-sandbox
--bins` for the gate's profile inside the same hold, as the wrapper's `--build`, then walks. And a measurement binary copied aside for an
A/B has no worker beside it: it draws no CCITT, JBIG2 or JPX image and says nothing — round 1361
read 157 M instructions for `bug1815476.pdf` with no fax decode in the profile. Copy the worker too,
or set `PDF_SANDBOX_WORKER` (trap 99).

The rebuild is the wrapper's own: `tools/bounded.sh --lock --round <session> --tree 12 --build '--profile gates -p pdf-sandbox --bins' --`
builds inside the same hold, before the command and with the lane descriptors closed, so nothing a
sibling edits between a build and its walk reaches the walk; a build before the lock is the defect
this trap names (ADR 1710). A package's own binaries need no line, since Cargo builds them for its
integration tests. And a self-test's hand-off waits for the run under test to log that it queued,
never for a hold of so many seconds: two- and five-second holds failed at a load an idle machine
never showed.

### 111. `cargo fuzz run` writes into the main checkout, and the seed population is not 974 documents

`fuzz/corpus` and `fuzz/artifacts` are symlinks into the owner's checkout. `cargo fuzz run` creates
`fuzz/artifacts/<target>` there whatever `-artifact_prefix` it is given — round 1362 removed seven
empty directories it had made; `cargo fuzz build` and the binary run by path write only where told.
And `fuzz/seeds.sh`'s `documents` are every file under `corpus-cache`, `doc/corpora` and
`doc/pdf.js/test/pdfs` — tens of thousands of files, over a hundred gigabytes (`find -L` over the
three counts them) — so a recipe over the whole population writes about 46 000 seeds and 4.5 GB and takes an hour behind
the lock; every recipe keeps the smallest seed of each shape it names (ADRs 1559, 1571); the four
whole-document targets — `document`, `serialize`, `linearize`, `page` — are still about 1 GB each,
because the coarser shape that would be megabytes lost 4.6–4.8% of coverage on the sample.

### 112. A sanitiser build dies behind the lock, a pipe over `seeds.sh` never ends, and `mmap` lies about `in`

Three things round 1368 lost hours to. A fuzz binary built with AddressSanitizer reserves its shadow
memory before the first input and dies under `tools/bounded.sh --data`'s limit with a log that says
only "failed": build `-s none` for anything run behind the lock. A pipeline that reads
`fuzz/seeds.sh`'s output (`| grep`, `| tee`) never ends, because a recipe's `cargo build` starts an
sccache server that inherits the pipe's write end — redirect to a file and read the file. And a
Python `mmap` answers `b"/DCT" in m` with False for a multi-byte needle it holds; `m.find(b"/DCT")`
is right.

### 114. A nested crate joins the workspace, `__pycache__` is a member, and an export's mtimes are the commit's

Three ways one round's scratch broke every sibling's build in batch fifty-eight. `cargo new` under
`scratchpad/r<n>/` adds the new crate to the root `Cargo.toml`'s `members`, and `cargo` in the whole
worktree answers "multiple workspace roots" until the line is removed. Running `python3
tools/main-checkout.py` writes `tools/__pycache__/`, which the `tools/*` member glob reads as a crate
directory — `PYTHONDONTWRITEBYTECODE=1` on every Python run under `tools/` — and never `python3 -m py_compile`,
which writes the cache whatever the variable says; `python3 -c 'import ast,sys; ast.parse(open(sys.argv[1]).read())' <file>` checks a script without it (round 1382). And a `git archive`
export's files carry the commit's time, older than any artefact in a target directory that already
exists, so a rebuild over it hands back the stale binary with no error: touch the exported sources
before building and compare the binaries with `md5sum` (trap 50 is the same lesson from the other
side).

### 116. `cargo-geiger` is a fork bomb, and a fresh-device gate is not a window's device

Round 1377 was asked for a dependency's `unsafe` count and ran `cargo-geiger` 0.13, unbounded, in a
scratch directory. It forks a `cargo-geiger metadata` task per package and never waits: the agent's
cgroup climbed about 3 400 tasks a minute for ten minutes to 52 259 tasks, 50 GB resident and 91 GB
of swap, and the system's OOM daemon killed the whole scope — the orchestrator and four rounds
(2026-10-06, the incident class's fourth occurrence). Re-run under `ulimit -u 8192` and
`tools/bounded.sh --tree 12` it reached 7 845 tasks and 13 GiB in 39 s before the bound killed it.
Count `unsafe` with `grep -rwo unsafe <crate>/src --include=*.rs` (ADR 1590's figure is that), and
run every heavy command under the process limit and the bound. And `zoom_frame` and `frame_budget`
draw every round on a device made for the round: a change whose cost or saving depends on the
device's own state — wgpu's command-encoder pool, which allocates sixteen Vulkan command buffers on
first use and about 3 ms per encoder while the pool grows — reads as a loss on the fresh device and a
win by the sixth frame of a long-lived one (ADR 1595). Measure a sequence of frames on one device
(`ZOOM_FRAME_SEQUENCE`) before calling such a change either.

### 118. Boa's default module loader reads the file system, Landlock needs a descriptor, and a pass costs two command buffers

Three findings of the script worker's confinement and the encoder-pool measurement. `Context::builder()
.build()` installs Boa's `SimpleModuleLoader`, which calls `realpath(".")` at the first evaluation;
under a profile with no file descriptors every confined worker died of `SIGSYS` at its first run,
and the fix is `IdleModuleLoader` on every context (ADR 1608 — trap 31's shape, a library's hidden
first-use syscall). Landlock builds its ruleset through a descriptor, so a `RLIMIT_NOFILE` of 0 is
set after Landlock and never before, for every profile. And wgpu-core 30 allocates two HAL command
buffers per render pass, sixteen at a time: `bug1721218_reduced.pdf`'s first frame makes 17
allocations where a launch row's makes 2, growing the pool beside the first frame slowed the frame
(7.1 → 7.9 ms), and growing it deep enough for the heavy page costs 37 MB on every device — the
lever ADR 1594 priced was declined on the count (ADR 1606). Count the allocations by call before
pricing a pool.

### 119. A spliced doc comment, a count held at zero, and a self-test's abandoned processes

Three ways a round's instrument misled it in batch sixty. A doc comment for a new function spliced
in front of an existing `fn` line lands between that function and its own doc comment; anchor the
splice on the blank line before the doc comment (round 1384). A Rust rule that counts a shape in a
document is cross-checked against an independent script copy before its count is held at zero —
the script caught the possessive "session's" and "a hundred sessions" that the rule missed, and
the rule was widened (ADR 1610). And a self-test case that leaves processes behind on purpose runs
in an `exec`-redirected subshell: a redirection on a function call leaves the caller's saved
descriptors open in every subshell it forks, and `cargo test`, reading through a pipe, waited 71 s
for 15 s of work (ADR 1612).

### 121. A column prints its change per site, a per-pass saving is read per pair, and a figure carries its unit

Three measurement habits from batch sixty-one. When a column gains a site — Table 200's five
triggers joined the Tier 1 column — the change is printed per site, so the gate's ceiling is seen to
have moved only by the new site's runs: +18 throws, all Table 200, the other sites at exactly 9 575
(round 1389). A saving of 29 render passes was 0.6–1.4 ms on a 152 ms frame, which `zoom_frame`'s
spread hides: the two trees are interleaved run against run and the direction read per pair before
it is called a saving or noise (ADR 1618). And a record's `script_open spawn_ms=1.8` was read as
1.8 s in a brief, measured as 1.8 ms, and a round's premise was off by a thousand (ADR 1620): a
duration is quoted with its unit, and "s" is written when seconds are meant.

### 122. A time-bounded target at nice 19, a scanner over "the rest", and a clock under the lock's prefix

Three instruments that lied in batch sixty-two. A fuzz target bounded by wall time, run under
`tools/bounded.sh`'s default nice 19 beside siblings' builds, failed on the seed that sits at its
budget (`new Array(1048576).fill(0)`: 2.1 and 3.4 s against 2 s, 0.57–0.68 s alone) — a
time-bounded target runs with `--nice 0` (round 1395). The same round's depth scanner decoded
the rest of the script for every character and was quadratic; the fuzz campaign found it in a
64 KiB string compiled at run time, and a four-byte window fixed it (ADR 1626). And the lock
prefix `RAYON_NUM_THREADS=4` reaches whatever runs inside it: `zoom_frame`'s CPU backend read
93.7 ms under it where it is 47.6 — a clock child sets its own pinned core count, as `turn_path`
does with `RAYON_NUM_THREADS=8` and `taskset -c 0-3,12-15` (round 1397).

### 124. An export labelled HEAD is HEAD's only if it is one binary from a clean tree

The orchestrator's first hand export of a batch's corpus arms ran `cargo test` per arm in the
shared worktree: the second arm recompiled a sibling's `pdf-model`, the "digests" captured were
the gate's summary (the per-page digests go to the file `PDFVIEWER_RASTER_TIMES` names), and a
sibling's mid-edit `tools/bounded.sh` turned a passing arm into `exit 1`. `tools/batch.sh arms`
(ADR 1650) refuses a dirty tree, builds the corpus binary and the worker once, copies them and
writes their SHA-256 into the README, and runs all six arms under one hold of the lock. And a new
variant on an enum another slot's file matches exhaustively breaks that file at once: `Piece::Tab`
broke `popup/rich.rs`, the sibling adapted, the variant was briefly removed and broke it again
(round 1406). Grep the matches outside your files before adding or removing one, and settle on
one shape.

### 128. A detached job is backgrounded alone, after a `cd` and a `;`

`warm` and `arms_at_open` started their jobs as `(cd "$wt" && setsid nohup … &)`. Because `&` binds
looser than `&&`, the `&` sent the whole list to the background: a copy of the shell ran the job in
its own foreground with its standard streams on the caller's pipe, and every caller that read the
pipe to its end — a command substitution, an agent's shell — waited out the arms export's half hour
(ADR 1662). The construction is `cd … ;` first, then the job as the only thing backgrounded, every
stream redirected. The test is a caller that reads the pipe to its end against a stand-in job that
sleeps: the old form returned after the sleep, the new one in milliseconds.

### 129. An export built under the shared worktree builds into the shared target directory

A copy of HEAD placed under the worktree picks up the worktree's `.cargo/config.toml`, whose
`target-dir` is the batch's shared build directory. Its units land there and look fresh to a sibling's
next build of the same unit, because cargo's dep-info paths are relative and the files match by
mtime — so a sibling's next build of that unit comes from the export's tree. Round 1410's first
probe build did this to a `pdf-model` fingerprint and deleted it within minutes. Every export's build
gets its own `CARGO_TARGET_DIR`.

#### 50. Two exported trees sharing a build directory can hand you the same binary twice

cargo hashes a path dependency relative to the workspace root, so a "before" export and an "after"
export under one `CARGO_TARGET_DIR` reuse each other's artifacts by mtime, and the A/B's first pass
produced two identical binaries (ADR 1311). Give each export its own target directory, or touch the
overlaid sources, and `md5sum` the two binaries before quoting a difference.

### 130. A running shell script is replaced by rename, never edited in place

Bash reads a script from the file as it runs, so an in-place edit shifts the offsets the running copy
reads next. Round 1409's mid-run fixture rename in `tools/drive-windows.sh` ended the first drive in
"syntax error near unexpected token `done'" after 1 057 s; round 1411's rewrite of `tools/bounded.sh`
made its own running wrapper exit 2 after the command it had waited for. Write a new file and rename
it over the old one, and keep the exec bit: a Python rename lost it, and the queued run cost 3 929 s
of queue for an exit 126. The side lesson from the same drive: a new fixture's name must not reuse an
existing control's, which it silently overwrites — the control's step turned wrong.

The third instance: round 1464 edited `tools/drive-windows.sh` while a whole drive of it held the
lock, and the run died at line 3152 with a syntax error after every verdict had been written. A
script is never edited while a run of it holds the lock; the edit goes to a copy, or waits for the
hold to end.

### 131. A daemon started inside the lock keeps it after the walk

An `sccache` server started by a `cargo` command inside round 1412's hold inherited the wrapper's
descriptor on `heavy-walk.lock` and held the lock about 18 minutes after that wrapper had ended,
until it idled out; rounds 1408 and 1411 queued behind it (ADR 1659). When the lock is held and no
wrapper is running, `ls -l /proc/*/fd` for the lock's path names the holder; the shared server is
never killed, and it is started once outside any hold. The wrapper hands its command
`HEAVY_WALK_HELD_BY` in place of the descriptor, which is how `arms-held`'s check (ADR 1662) still
finds the hold (ADR 1674).

### 133. A `static` table of references is paid at load, by the loader, before `main`

Round 1436 compiled the OpenType language-system registry into a `static` whose entries were
`&[[u8; 4]]` slices. Nothing read the table at open, yet two launch rows' `open_kinstructions` rose
past their ceilings: in a position-independent executable each reference in a `static` is an
`R_X86_64_RELATIVE` relocation the dynamic loader applies before `main` — 1 023 more of them, about
12 instructions each, 12.3 k instructions in the loader's own function (ADR 1708). The fix is the
shape, not the data: a flat array of plain numbers and an index of code, start and count, sliced at
lookup time, needs no relocation. `open_kinstructions` is the gate that sees this and no clock does;
when a compiled-in table arrives, the counted launch gate is run before the clocks.

### 135. Seeding a member is not reaching it

Round 1463 bridged the tail of Tier 1 — `event.shift`, `modifier`, `keyDown`, `richValue`,
`richChange`, the word pair — and the brief asked round 1471 to seed the `script` target so the
campaign would reach them. No seed could: the target's realm told every script that every key was up,
carried no rich value and had pages without words, so the members' answers were constants whatever
the seed spelled, and libFuzzer had no edge to find (ADR 1776). The input now carries an optional
fourth part — the keys and a rich value — and the realm's pages have words. Before a member is called
covered, check that the target lets the input the member reads *vary*, and confirm it the way trap 13
confirms a test: take the input away and watch the edge count fall.

### 136. A foreground `flock` holds back every trap until it is granted

A clock run waited for the gate in a foreground `flock`; bash defers a trap until the foreground
command returns, so a TERM sent while the run queued did nothing until the lane was granted, and
then the wrapper exited on no line (round 1477, ADR 1790). The wait now runs in the background under
`wait`, which a signal interrupts, and a stopped wait writes `hold=0.0s exit=<status> lane=-`. The
test of any wrapper that must speak on a signal is the signal sent *while it waits*, with the line
timed against the moment of the signal and not the end of the hold.

### 138. A budget's fixture is checked for still reaching the budget

`MAX_LIST_BYTES`'s test built a Type 3 glyph that showed itself, which reached the list-bytes bound at
HEAD; the glyph re-entry refusal (ADR 1792) stopped the same fixture at 44 commands, and the test
would have gone on passing on a refusal it was not about (round 1478). A hostile-budget test asserts
the bound's *own* report, so a fixture that stops reaching the bound fails by name, and the fixture
moves to a route that still reaches it — here a form cycle through a tiling cell, which ADR 0793 bounds
by depth alone.

### 139. A census is declared `--long` only once a log line shows its peak

A `--long` run holds the second lane, whose ceiling is 6 GiB, and is refused above it. Round 1478's
nesting census over 90 763 first pages had never run on that population; declared `--long`, it was
killed at 6.77 GiB after 828.5 s and then queued 844.8 s to run again as a large walk, so the hold was
paid twice (round 1485, ADR 1806). A census not yet run on its population is declared large, and is
declared `--long` only once a line of `/home/AI/heavy-walk.log` shows its peak under 6 GiB.

### 142. A reading on the owner's display is taken from inside a clock hold

The owner's loop runs a script on their display whenever one appears, beside whatever walks hold
the heavy-walk lock at that moment; it knows nothing about the lock. Round 1489's first after arm
ran beside a sibling's six corpus arms at a load average of 11.3 and read `quorra`'s first frame on
the witness at 32.94 s against 22.08 s at HEAD; retaken from inside a `--clock` hold, with both
arms in the one hold, it read 12.33 s (ADR 1815). A script for the loop is written and waited for
from inside a clock hold, and a before/after pair goes through the loop inside one hold.

### 144. A partial picture gets its own trace line, and "first frame" keeps meaning the finished page

The toolkit windows now show a page as it is built, 0.5 s into a long draw and every 250 ms after
(ADR 1825). Had that picture been traced as the first frame, step 80 and `tools/state.sh heavy`,
which grep that line, would have started timing a half-drawn page without anybody changing them
(round 1494). A partial picture logs "page N shown as it stands, P % drawn"; the first-frame line
means the finished page; and an instrument that reads a trace line is re-read whenever the line's
meaning could move.

### 147. A refusal's byte figure is traced to its site, not factorised

The device refused the witness at 423 206 976 B, which is 4 408 406 marks × 96, and 96 = 6 × 16 read
as six flattened edges of 16 B each (ADR 1817). It was the encoder's pre-count — one rect instance
and one quad instance, 32 + 64 B, charged to every command before any edge exists — which is why the
refusal came with the encode at 0 ms and the scene at 2.1 s (round 1493, ADR 1823). A refusal's
figure is traced by which phase was still at zero when it came, and the ADR that prices a lever on
it names the site.

### 149. A clock hold keeps the lanes, not the machine

A `--clock` hold gives a run both lock lanes; it does not keep other rounds' builds off the
processors. Round 1507's hold ran at load averages of 24 to 52 from siblings' builds outside the
lock, and every clock figure in it was declined as "not judged" (ADR 1851). A round's own clock A/B
judges anything only when no sibling is building; a quiet reading comes from the merge's hold, where
nothing else builds, or is taken with the run's printed load average beside it after a build-free
moment has been waited for.

### 153. An export's copy of a shared file is HEAD plus your own hunks

Round 1511 copied `renderer.rs` from the shared worktree into its export of HEAD plus its patch, and
the copy carried slot 1's in-progress edits; the export did not compile (ADR 1860). A file other
slots also edit is rebuilt in an export as `git show HEAD:<file>` with your own hunks applied, never
copied from the worktree, and the export's `git status` against HEAD names only your files.

### 157. A log's first line saying "queued" is the moment it queued

A wrapper prints "queued for the heavy-walk lock" when it asks and nothing when it is granted, so a
log whose first line says queued may belong to a walk that has been running for ten minutes. Round
1512 stopped its own `gpu-4x` arm at the budget on that reading, 649 s into its run (ADR 1861). A
wait is stopped only after `/home/AI/heavy-walk.log`, `<lock>.queue` or the walk's own output has
said it is still waiting.

### 158. A share found by counting instructions is re-taken on the clock before a lever is sized from it

Lexing was 35 % of the witness's read by callgrind (ADR 1833), and batch eighty-two's brief sized a
3× lever on that share; on the clock, after ADRs 1849 and 1861 had cut beside it, lexing was 18–19 %
and the lever's ceiling 1.24× (round 1518, ADR 1873). An instruction share decays with every cut
made beside it, and the clock is the unit a lever is priced in. Count instructions to find where the
time goes; re-take the share on the clock before a build is sized from it.

### 159. A rule that frees a lane while a claim waits also stops counting the tickets it cannot grant

ADR 1869 lets small walks onto the second lane while a clock run waits for the first. The brief's
hypothesis missed that a `--long` run's earlier ticket, which can never be granted while the clock
turn is held, would have kept the second lane idle under the new rule exactly as under the old one
(round 1516). A rule about who may be granted is also a rule about which tickets count, and the
self-test plants that case (`bounded.sh --self-test`, case 9, run 99) so the rule cannot be called
built without it.

### 162. An export links every gitignored path, not only the corpora

Round 1520's export of HEAD plus its hunks was a `git archive` with the corpora linked in, and
`turn_path` in it failed with "measured nothing": the gate's own document, `doc/PDF20_AN001-BPC.pdf`,
is gitignored and the archive leaves it out, so the clock hold was spent on a child that opened
nothing. Trap 150 names the untracked documents and `tools/batch.sh open` links them for a
worktree; a round's own export does the same from `git status --ignored`, every path it names, before
the first gate runs in it. `tools/batch.sh export <session> [<path>...]` does, and its record in
`scratchpad/r<session>/export.export` counts what it linked (ADR 1890).

#### 150. An export made with `git archive` has no corpora

`git archive` writes the tracked files; the submodules and the symlinked corpora (`doc/pdf.js`,
`doc/corpora/*`, `corpus-cache`, `fuzz/corpus`) come out as empty directories and the untracked
documents (`doc/*.pdf`) are not there at all, and a gate run from
such an export skips their pages and says so only in a line nobody greps — round 1502's first `turn`
run reported "10 page(s) absent" (ADR 1841). An export links the corpora in the way
`tools/batch.sh open` does before any walk, and a gate's "absent" count is read as a refusal.
`tools/batch.sh export <session>` is that export: HEAD's archive with every submodule and every
gitignored path linked in (ADR 1890).

### 163. An export under the worktree is patched with the repository search stopped, and conformance runs in the worktree

Three rounds of batch eighty-three patched an export kept under `scratchpad/r<n>/` with `git apply`
and measured HEAD twice: run inside a repository's subdirectory, `git apply` silently skips every
path outside that subdirectory, and the "after" binary had HEAD's md5 (rounds 1522, 1523, 1527 —
trap 50's tell from a new cause). `tools/batch.sh export` applies under `GIT_CEILING_DIRECTORIES`
(ADR 1890). And `cargo test -p conformance` cannot run in a `git archive` export at all: its sweeps
read git's tracked files and fail with "the population is not the tree", and `undenominated` fails
until the corpora submodules are linked (round 1526) — it runs in the worktree.

### 164. A lock line's clocks are wall time, and a suspend is in them

Round 1524's lane-2 build wrapper reported `hold=10178s` for a build cargo timed at 350 s, and its
six arms 11 412 s held, while round 1522's clock run waited 11 120 s behind them and was killed at
its budget: the machine had slept about 9 800 s. The log's `hold=` and `wait=` are wall clocks and
include a suspend, so a huge hold may be the machine sleeping and not a walk. Read
`CLOCK_BOOTTIME − CLOCK_MONOTONIC` over the interval before blaming a run or a neighbour.

### 169. A hold checks its binaries exist, and a directory for the owner's loop is the owner's group

The clock turn outranks a build's ticket, so a clock hold queued after its own lane-1 build can be
granted first: round 1529's first hold measured binaries that did not exist yet and spent one of the
owner's-loop runs on nothing. A hold script asserts its binaries before it measures or queues the
loop. Round 1531 made `tmp/r1531` with group AI instead of `coders`, so the owner's loop could not
write its exit file and the hold waited 900 s holding both lanes; a wait on the loop watches
`run-on-gpu.exit` as well as its own done line, and the directory is made `chgrp coders; chmod g+ws`.

#### 145. A directory for the owner's loop belongs to group `coders` with the setgid bit

The owner's loop runs as `cl`; a directory this user makes under `tmp/` is in group `AI`, so the
loop's script cannot write into it. Round 1495's first owner's-display run wrote nothing and
reported "never" for every figure instead of an error (ADR 1827). Before a script is placed,
`chgrp coders` and `chmod g+ws` its directory, and have the script fail loudly on any write it
cannot make, so a missing figure is a refusal and not a number.

### 171. A figure moved by a per-mapping kernel policy is fixed in the child, not guarded by a counter

`peak_anon_mib` fell under its floors on HEAD while nothing in the tree moved it. The cause was
transparent huge pages: which of a child's mappings got a 2 MiB page depended on where each landed,
which no machine-wide counter reports — with every block free and `thp_fault_fallback` still, two
children read 19.45 and 19.63 against a floor of 19.5 (round 1533, ADR 1902). A guard that declines
on a counter would have judged them. The instrument turns the policy off in the measuring child
(`prctl`, through `rustix`) and reads back that it did; the ordinary children keep the machine's pages.

### 173. A race measured under the lock is measured with the lock's environment

Round 1535's hang — `quorra-gtk` starting without a session bus, the decoder worker inheriting GLib's
`dbus-launch` pipes — came up in 1 of 12 starts under the lock and 7 of 16 outside it, because the
wrapper's `RAYON_NUM_THREADS=4` narrows the window in which the worker's `posix_spawn` lands inside
GLib's; the drive's private bus and AT-SPI's bus on its display hid it entirely (ADR 1907). A window
under test for a race runs with the lock's variables taken out of its environment, the way trap 122
takes the drive's bus out, and the drive step that pins it (step 84) does so.

### 175. An ignored test or a moved file is settled in the round that adds it, not at the merge

`tools/conformance`'s sequence check reads git's tracked files: an `#[ignore]`d test in a file nobody
tracks yet only *warns*, and a file `git mv`'d then unstaged is still tracked and gone. Both pass every
round's own gate and fail the merge's tier 1, where the staged population is read for the first time:
batch eighty-four's `viewer-host/tests/drawn_ahead_in_strips.rs` had no gate line, and batch
eighty-five's `image/cut.rs` and `images.rs` were tracked and absent. A round that adds an ignored test
writes its `doc/todo/02` §2 line, its `tools/batch.sh gates` entry and its `tools/state.sh` section in
the same round, requires the sandbox as every corpus gate does, and runs
`git add -A <its files>; cargo test -p conformance --test state_sections --test sandbox_gates` to
see what the merge will see.

### 177. A threaded census under massif runs with `--fair-sched=yes`, and a peak is `os.wait4`'s

massif's default scheduler runs one thread at a time and let the pipelined read's calling thread
starve: nothing was released until the read ended, and the cut that lets each segment go read a
*higher* peak than HEAD (1 590.2 against 1 581.8 MiB) while a native run released 91 segments during
the read (round 1544, ADR 1924). A census of a threaded pipeline runs under `--fair-sched=yes` and
its order is confirmed natively; and this machine has no `/usr/bin/time`, so a process's peak is
read with `os.wait4` in a Python wrapper (round 1544's `py/`).

### 180. An inline quotation in a Rust doc comment is no gate's population

`every_quotation_is_the_standards_own_words` reads rustdoc blockquotes, and `--bin quotations` reads
Markdown and the ledger's notes. A `"…"` quotation written inline in a `///` or `//!` comment is read
by neither: round 1551 planted a misquotation inside one and both runs stayed green. A sentence of the
standard that matters goes in a blockquote under its clause number, where it is checked; an inline
quotation is prose that claims to be a quote, which principle 5 forbids unless it is verbatim and
somebody looked.

### 183. A sweep for a lint lift that matches the attribute's exact text is blind to the multi-line form

`viewer-qt`'s sweep for `unsafe_code` lifts would have passed with three new lifts in the tree,
because each was written as `#[expect(unsafe_code, reason = "…")]` over several lines and the sweep
matched one spelling on one line. Round 1554 made the sweep strip white space and match up to the
lint's name, and planted a lift back to see it fail (ADR 1945). The general rule: an attribute's
text is not its identity; a sweep that matches text is calibrated by planting the thing it hunts.

### 184. A still clock reads as a slow engine

Round 1557 ran Doom's start-up for 600 s and then 3 600 s without it ending, and concluded the
engine was a thousand times too slow. It was waiting on `Date`: the worker's clock gives one moment
for the whole run, and the program spun until that moment moved. With a clock stepping 29 ms per
read, start-up ended in 0.87 s and a tick measured 191 ms against the 28.6 ms needed (ADR 1950).
Before an engine is priced from a run that did not finish, the clock is given a step; a run that
then ends was never slow.

### 185. A re-export replaces the tree under a build queued in it

Round 1557 re-exported its session while a build it had queued under the lock was still waiting
for its lane. When the lane came, the build's working directory had been replaced, and `cargo`
failed with `Could not locate working directory` — one thirty-minute queue lost. An export's tree
belongs to every command queued against it until that command has run; a second export of the same
session is a new name (`--as`), or it waits.

### 187. `doc/md` in a worktree is the owner's directory

Round 1561 followed `doc/third-party-data.md`'s recipe — `tools/spec-md.py … --out doc/md/X.md` —
and wrote OGC 01-009's conversion straight into `/home/cl/projects/pdf-viewer/doc/md/`, because the
worktree's `doc/md` is a symlink there (`tools/batch.sh open` links every untracked document in).
The round noticed and removed it, and main's `git status` matched the session's start. The rule
every brief states — never edit the owner's checkout — is not a rule about paths that *look* like
the worktree's; before writing under `doc/`, `readlink -f` says whose directory it is, and a
conversion a round needs goes under its own `scratchpad/`.

### 189. A Python heredoc swallows Rust's string continuations

Round 1562 wrote a window's printed sentences through a Python `'''…'''` string. Rust's `\` at a
line's end inside a literal is the continuation that drops the newline and the next line's leading
spaces; Python's non-raw string consumed the backslash first, so the compiled text joined its words
with the indentation's spaces and said so in the windows before anyone read it. The check is one
grep over the diff for a literal holding eight or more spaces in a row; the cure is a raw string, or
a shell heredoc with a quoted delimiter, for any file whose language has its own escapes.

### 191. A build into the shared target replaces the binary a sibling is driving

Round 1560 rebuilt `quorra` in release into the shared batch target while round 1559's drive was
running the `quorra` that had been there. A drive names its binary by path, and the path is the
shared target's, so the sibling's measurements from that minute were taken on a mix of two programs.
The round had already copied its own binaries into `scratchpad/r1560/bin-new` for its own runs; the
rule is that every drive does, and that a release build into the shared target is announced in the
lock's log line. The export (`tools/batch.sh export`) exists for the same reason and is the better
tool: it builds under a target of its own.

### 192. The harness's scratchpad is shared by every round of the batch

Round 1564 wrote its `tools/round.sh` output to the harness's session scratchpad and, minutes later,
read round 1569's: the six rounds of a batch are subagents of one parent and the harness gives them
one scratch directory. The worktree's `scratchpad/r<session>/` exists so that a round's files are its
own; a path outside it is shared with five others whatever its name says.

### 193. A shared worktree's diff is nobody's patch

Round 1568 had to export HEAD plus its own change for a drive while round 1565 was editing the same
`host.rs` files and `native_window.rs`. `git diff` interleaves both. The round filtered `-U0` hunks
by the sibling's markers, applied them to HEAD in a private index and diffed at `-U3`; two things bit
on the way — a replacing hunk split in half, and a pure insertion whose new-side line number still
counted hunks that had been dropped, so the forwarders landed one line past the impl's closing
brace (ADR 1972). A patch assembled from a shared tree is read back line by line before it is built.

### 194. A wrapper killed after its grant leaves its child under the lock

Round 1566 killed its own queued wrapper seconds after lane 2 was granted. The wrapper died, the
lock's line wrote `hold=13.3s exit=143`, and the child kept building for about four minutes under a
lane the lock now believed free, so a sibling's walk could have been granted the same lane over it.
A wrapper is a process group; `kill -- -<pgid>` takes the child with it, and only before the grant
is there nothing of the child to take. After the grant, the cheap thing is to wait.

### 195. `python3 -I` writes a `__pycache__` the workspace reads as a crate

Round 1570's one-off script imported `tools/cache-census.py` under `python3 -I`, and for six minutes
every cargo command in the shared worktree failed on a crate called `__pycache__`. `-I` isolates the
interpreter from the environment, `PYTHONDONTWRITEBYTECODE` included, so the bytecode cache the
environment forbids is written anyway. `-B` forbids it on the command line, where `-I` cannot undo it;
the census's usage line says `python3 -I -B`, and so does every script under `tools/` that another
script may import.

#### 69. A Python import under `tools/` leaves a directory that the workspace reads as a crate

Calibrating `comment-history.py` by importing it wrote `tools/__pycache__/`; the workspace member
pattern `tools/*` then read that directory as a member and every `cargo` command in the shared
worktree failed on its missing `Cargo.toml` (round 1289). Run `tools/` Python with
`PYTHONDONTWRITEBYTECODE=1`, or remove the directory the moment it appears.

### 196. A wall clock below the poll measures the processor, not the lock

`tools/bounded.sh --self-test` failed in three merge chains in a row and passed alone each time.
Round 1573 found the test judging "waited for nothing" by `wait=` against one 0.5 s poll, where `wait=`
on a lane granted at first look is the wrapper's twenty forks and a Python start — 0.0 s on an idle
machine, 5.3–13.8 s beside busy loops at nice 0, and `behind=-` on every one of those lines, so the
lane had been free. The lock writes whether a look found its lane held; that field is the fact the
test wanted, and the five sites that compared a clock now read it (ADR 1982). A test of a lock that
reads a clock is a test of whatever else the machine is doing.

### 197. Two drives' photographs differ by the drive's own names

Round 1576 compared 221 photographs of HEAD against HEAD plus its patch and found 46 different.
Thirty-one of them were the status strip printing the drive's output directory, a loopback port or an
answer file's temporary name — text that differs between any two runs and says nothing about the
program. The rest were a caret's blink, a window not yet up, a loading bar's moment, a focus ring, and
a face not loaded 0.4 s after launch (ADR 1406). A photograph comparison is read after its known
sources of difference are taken out, and the drive's own names are the first of those.

### 199. A gate binary copied out of `release/deps` judges nothing and passes

Round 1572 copied the turn gate's binary into its scratch, as trap 191 asks of every drive, and
the gate exited 0 with every figure "not judged": it reads whether to judge from its own path, and
`release/deps` was not in it. The second hold ran the copy from `<scratch>/release/deps/` and judged
33 of 33. A gate that can decline to judge says so in its output; a reader of a green exit looks for
that sentence before believing the green.

### 201. A Miri log names the first site, not the last

CI's nightly log stopped at `display_list.rs:890`, the pooled drop; guarding it let Miri run on to
`reach::tests::the_share_a_window_keeps_of_a_short_list_is_its_selection_counted`, a test that had come
in with ADR 1994 after the failed run and reached rayon through `MarkBounds::measure`. Round 1587 gave
it the `cfg_attr(miri, ignore = …)` its neighbours carry and ran CI's whole line — `cargo +nightly miri
test -p pdf-render -p pdf-syntax --lib` — to exit 0, 2 841 s of it (ADR 2010). The interpreter reports
one site per run; the job is judged by the run, not by the site.

### 202. `rustfmt` on a root formats the children too

Round 1585 ran `rustfmt` on `pdf-archive`'s `lib.rs` and found it had reached `table/document_level.rs`,
a file a sibling was editing in the same worktree; the sibling's changes were content and survived,
but the next time they may not. `rustfmt` follows `mod` declarations, so a root file is the whole
crate. Scoped formatting means one file at a time, the files one owns, and a `git status` afterwards
that shows no file one did not mean to touch.

### 203. An ordered list's later item is priced after the ones before it

Batch ninety-three's brief sent round 1586 to build `doc/todo/75` item 3 on the strength of "only
9 474 of 4 408 406 commands change with the view"; that remainder is what item 2 leaves, and item 2 was
not built. Today 40 of a line instance's 60 bytes are the device's, and a 1.25× step culls a quarter
of the admitted lines, so every step is the whole walk (ADR 2008). A figure quoted from a list is read
with the item that produced it; the orchestrator writes the brief's premise from the list's sentence,
not from the number beside it.

### 204. Two walls in one call add up to a hang

Round 1590 reported that `tools/wait.sh --start` around a locked command twice did not return. Round
1594 read the transcript: the harness had backgrounded both calls, one because `--start`'s own 240 s
wait exceeded the default 120 000 ms timeout, the other because a `--start` and a following wait in
the same call summed to 480 s against a 270 000 ms timeout. Nothing hung; the call outlived its
budget. `--start` now returns at once (ADR 2025), and the rule is general: one Bash call carries one
wait, sized under the timeout it was given.

### 205. A `sed` whose replacement holds its delimiter writes a stray file

Round 1595 rewrote a line with a `sed` substitution delimited by bars whose replacement held an
escaped bar; `sed` took the bar as the end of the substitution and the rest as a `w` flag, wrote a
file named after the remainder into the worktree's root, and left the line half replaced. The round
found both, deleted the file and repaired the line, and the merge's `git status` is what would have
caught them otherwise. An edit to a shell or Rust file is a script with an anchor it asserts on, as
every placement in this tree is written.

### 206. `git apply` in a subdirectory skips the paths above it and says nothing

Round 1597 applied its patch inside its export directory and `staging.rs` stayed at HEAD: `git apply`
restricts itself to the current directory's subtree when run below the repository root and reports
success for what it skipped. The export's figures were taken on a tree that lacked one of the round's
files until the arms' comparison was re-read. Inside an export, `patch -p1` applies what it is given;
either way, every path the patch names is diffed against HEAD before a figure is believed.

### 207. A detached run reads no standard input

Round 1596's first census, fed the file list through a pipe into `tools/bounded.sh`, reported 0 of 0
files and read as clean: the wrapper then ran its command in a background list that bash gives
`/dev/null`, and round 1604 made it pass the caller's input through (ADR 2044). The detached start
`tools/wait.sh --start` reads `/dev/null` by design (ADR 2025), so a pipe into a run started that way
still reaches nothing. A run long enough to need the wait takes its input from a file argument, and a
probe that can report "nothing found" is first shown something it must find.

### 208. A response's parallel waits run one after another

Round 1598 obeyed trap 204 to the letter — one wait a call, each under the harness's timeout — and
issued three of them as parallel tool calls in one response; the harness ran them in sequence, and
the model sent nothing for 354 s. The cache survived by luck (ADR 2039 counted every such gap a hit).
A wait is a thing a response does once: the call returns, the model reads it, and the next wait is
the next response's. The same arithmetic that bounds one call bounds the response.

### 211. `cargo tree` reads one platform unless told otherwise

ADR 2048 wrote the rule that the application and the C ABI depend on no widget toolkit as a `cargo
tree -e normal,build -i <toolkit>` command; round 1609 planted `cxx` under `cfg(windows)` in
`viewer-ffi` and the command printed "nothing to print" and exited 0. `cargo tree` resolves the host
target unless `--target all` says every one, and a check that reads one platform's graph proves
nothing about a host written for another. ADR 2055 amends the command and the test calibrates itself
with a platform-gated plant.

### 220. A hand-off patch is applied with context once a sibling has moved the lines

Round 1627 handed the orchestrator five `-U0` hunks for the archive slot's files, including a new test
for `tests/archive.rs`; round 1623 had meanwhile added lines above the insertion point, and `git apply
--unidiff-zero`, which has no context to anchor on, put the test's body inside another test and its
`#[test]` on a `let` — a compile error that tier 1 found after the gates had already spent forty
minutes (ADRs 2090, 2082). A `-U0` hunk is correct against the tree it was cut from and silent about
any other; when the file has moved under it, apply the hunk with three lines of context or by the
anchor the round names, and compile the crate's tests before starting the chain, so a misplaced hunk
costs a minute rather than a chain.

### 221. A sanitised fuzz target cannot run under the bounded wrapper's data limit

AddressSanitizer reserves its shadow memory as one enormous mapping at start, and `tools/bounded.sh`'s
`RLIMIT_DATA` refuses it; round 1633 lost 730 s to a build that could never have started before running
the JBIG2 target with `-s none`, as `doc/verify.md` already says and `tools/fuzz.sh` does not yet do
(ADR 2102). Two rules, which `tools/fuzz.sh` applies itself under a data limit (ADR 2127): a fuzz
target under the lock is built with `-s none`,
and it is started from a scratch directory, because `-jobs=1` writes `fuzz-0.log` into the current one.

### 227. A gate that calls the library sees nothing the binary around it does

Batch 101's brief named `t3-foreign_corpus` as the gate that would prove the transform suite's worker
split changed no output byte; the gate calls `apply` in-process and never runs `quorra-transform`, so it
could not have seen the broker, the worker or the fences at all (round 1652, ADRs 2140, 2141). The round
built the instrument the change needed — a digest walk of the binary over twelve verbs and eighty-two
documents, HEAD against HEAD plus the change, by the hash of status, output and every file written. Before
a gate is cited for a change, read what it drives: a library gate for a library change, a binary walk for
the program around it.

### 230. A stale transform worker runs the old archive code without a word

Since ADR 2140 the transform suite parses in `quorra-transform-worker`, and `cargo build --bin
quorra-transform` rebuilds the broker alone: round 1655's first witness conversion ran the previous
round's archive code inside the stale worker and printed the old refusal, with nothing saying the two
binaries disagreed (ADR 2146). Trap 10's shape, now for the suite's own worker: build the package with
`--bins`, and when a verb's answer looks like yesterday's, check the worker's build time before the code.

### 233. A re-export replaces the directory a queued walk of yours will run in

Round 1706 queued a cross-check inside its export, re-exported the same name to pick up an edit while the
walk waited on the lock, and the walk died with "Could not locate working directory" after the whole
wait: `tools/batch.sh export <n> --as <name>` removes and rebuilds the export's tree, and a process whose
current directory was inside it has no directory any more. Re-export only when no walk of yours in that
export is queued or running; otherwise make the edit in the export's copy and in the worktree, identically.
Related: `cargo test -p conformance --test bounded` reads git's file population and cannot pass inside an
export, which has no index — run it in the worktree.

### 235. `bounded.sh --build` builds where the wrapper starts, not where the gate runs

Round 1707 ran `raster_golden` inside its export, the wrapper asked to build the sandbox worker with its
`--build` option, from the worktree's directory; the worker was built into the worktree's target, the export's gate
looked beside its own binaries, found no worker and refused loudly (trap 10 caught it, ADR 2248). The
wrapper's `--build` runs `cargo` in the directory it is started from: a walk of an export is started with
the shell already inside the export, and the first line of its log — the build's target path — is read
before the walk is believed.

### 237. A `--clock` hold stops locked walks only, and a loop figure carries the load average beside it

Round 1716 measured the Qt wheel chain on the owner's display inside one `--clock` hold and read the
same binary's scene encode at 6.2 ms in one arm and 52.7 ms in another a few minutes later: seven sibling
rounds' bare `cargo` builds — crate-scoped checks, which habit 107 rightly keeps off the lock — had the
machine at a load average of 30 to 44 on 24 threads (ADR 2266). The hold excludes other *walks*, not other
work. A figure taken on the loop or in a clock hold is written with the load average beside it, a quiet
arm (load under 4) is repeated before any term is attributed, and a figure whose spread exceeds its
quiet reading is the machine's until a quiet arm shows otherwise.

### 238. A clock hold is granted ahead of the build it depends on

`tools/bounded.sh --lock --clock` takes both lanes and is granted before queued walks; round 1712 queued
its A/B hold while its export's release build was still waiting on the lock, the hold ran first, and
every arm of the round's own tree exited 127 for want of a binary while HEAD's arms ran — one whole hold
wasted (ADR 2259). A clock hold is queued only once `ls` shows every binary it will run, and the script it
runs checks for each at the top and exits, naming the missing one, before the clock is held.

### 240. A bring-up figure under a display is not the headless gate's figure

Round 1716 read the device's bring-up on the owner's session at 135–227 ms and round 1724 took it apart:
26–31 ms is the loader and the drivers, which is what the headless gate measures; under a display Mesa's
`VK_LAYER_MESA_device_select` asks the compositor on both of the launch's enumerations and adds 13–25 ms;
on an idle Xvfb each of the four X connections a window opens waits 38–56 ms for the server's setup reply,
145–220 ms in all; and the rest was the machine's load (ADR 2282, trap 237). A window's bring-up is compared
only with a child run under the same display variables, never with the gate's headless band, and the
`graphics device up in` trace line's per-step figures say which share is the display's before any term is
named. Since ADR 2298 the launch enumerates the adapters once, so the probe is paid once rather than twice.

### 242. An export's name is a directory the export owns

`tools/batch.sh export <session> --as <name>` builds the export under `scratchpad/r<session>/<name>/`, and
makes that directory its own: round 1720 staged HEAD's binaries under `head/` for an A/B and then exported
HEAD `--as head`, and the export emptied the directory first, so HEAD's figures in the record come from
the run made before it (ADR 2274). A round names its exports with names nothing else under
`scratchpad/r<session>/` uses, stages what it keeps under a name the export will never take
(`staged-head/`, `bin/`), and reads trap 233 before re-exporting while anything of its own runs there.

### 244. A patch for a shared file keeps only the hunks that name the round's own words

Rounds 1728 and 1733 both edited GTK's `input.rs` and `native_window.rs`; round 1733 made its `-U0` patch by
filtering the worktree's diff for the sibling's identifiers and keeping the rest, and the sibling's hunks
that used none of those identifiers stayed in (ADR 2300). A hand-off patch for a shared file is built the
other way round — keep the hunks that name the round's *own* identifiers, listed in its `touching.txt` —
and each kept hunk is read once against that list before the patch is handed over, because a filter by
the other party's words describes what the other party wrote, not what this round did.

### 246. A round's clock hold is queued before its own large walk

Round 1732 queued its six-arm walk and then its A/B clock hold, and the hold would have waited out the
walk's whole queue on the first lane before its own grant; it withdrew the walk and queued the hold first
(ADR 2298). Clock holds take both lanes and are granted ahead of waiting walks, but not ahead of what is
already running — so a round that needs both queues the clock hold first, once its binaries exist (trap
238), and its large walk after, and reads the lock's line for each to see the order it got.

### 247. `py_compile` writes a bytecode cache whatever `-B` says

`python3 -B` keeps the interpreter from writing bytecode when it *imports*; the `py_compile` module's whole
job is to write it, and `python3 -I -B -m py_compile` on a tools script leaves `tools/__pycache__/` behind, which
the merge check refuses (round 1728, ADR 2290). A tools script is syntax-checked by parsing it —
`PYTHONDONTWRITEBYTECODE=1 python3 -I -c 'import ast, sys; ast.parse(open(sys.argv[1]).read())' <file>` —
or by running it with `-I -B` on its own `--help`, and never by `py_compile`.

### 248. A source copied in with an old timestamp leaves cargo's old artefact in place

Cargo decides whether a crate is fresh by comparing its sources' mtimes with the fingerprint of the
last build, profile by profile; a file written into the worktree with a timestamp older than that build —
`cp -p` or `rsync -a` from an export, an archive unpacked with its times — reads as unchanged, and the
profile keeps the rlib it had. Batch one hundred and twelve's merge chain compiled the workspace clean
under the dev profile and then had seventeen gates fail to compile under the gates profile against a
`pdf-colour` and a `render-cpu` from the day before, until `cargo clean --profile gates` for the crates
named and a rebuild. A round copies sources into the worktree without preserving their times, or
touches them after; an orchestrator whose gates fail to compile what tier 1 compiled cleans that profile
for the named crates first, before reading anything else into the failure.

### 250. "Build-dependent" across one batch's rounds is two source states, not two builds

Round 1734 saw a document refused by two builds of HEAD and converted by later builds of "the same
source", and called it build-dependent; round 1736 ran the converter twenty times from two builds of one
commit and got byte-identical output but for the recorded instant, and found the earlier builds had lacked
a sibling round's hunk from the same batch (ADR 2306). In a shared worktree "HEAD plus my hunks" is a
different source every hour. A claimed build dependence is reproduced first from two builds of one commit
in two target directories, then from one source with each sibling hunk taken back in turn; only a
difference that survives both points at a hasher, a pointer key or a thread order.

### 255. A file named whole in an export carries every sibling's hunk with it

`tools/batch.sh export <n> --as <name> <path>` copies the worktree's file, not the round's hunks of it;
round 1739 named `raster-gpu/src/startup.rs` whole and its arms ran with a sibling's adapter-selection
hunks inside, which happened to choose the adapter HEAD chooses with no selection variable set, so the
figures stood — by luck, not by construction (ADR 2313). Before a shared file is named whole, `git diff -U0
<file>` is read and every hunk confirmed the round's own; a file with a sibling's hunk goes into the export
as `--patch` of the round's own hunks (trap 193), and the record names which files went in which way.

### 261. A hold granted before its build is a hold on somebody else's binary

Round 1749 queued a `--clock` hold while its export's release build waited on lane 2 as a `--long` run.
Clock runs take lane 2 ahead of long ones, so the hold was granted first; its "built?" check asked only
whether the binaries existed, and a target seeded by `cp -a --reflink=always` from the batch's release
directory (habit 185) already holds every binary name, so the check passed on the seed's binaries and the
`frame_budget` "mine" arm measured the batch directory's build. The round stopped the hold by its process
group after 368 s and quoted nothing from it. A hold is queued only after every build it depends on has
ended, and a binary is called fresh by cargo's own word — a build that ends with no `Compiling` line —
never by its presence on disk.

### 262. A GPU figure taken while another process holds the render node is a figure of the contention

Round 1752 priced a toolkit notch's window on the Radeon 890M headless and found the device running at
one of two speeds in stretches lasting seconds: a window's emit and deposit took 0.8–1.6 ms or 3.2–4.2 ms,
in every arm alike, and all six bare runs were slow while a sibling's corpus walk held the render node
outside the lock (ADR 2338). That spread is ADR 2323's 1.8–5.8 ms remainder — the device's state, not
the lane's. The lock keeps the walks apart but not the device: a bare GPU test or a window drive opens
`/dev/dri/renderD*` without it. Before an arm, the processes holding the render node are listed and the
list printed beside the figure; emit+deposit is read to say which speed the arm ran at; and a term is
attributed only across arms that ran at one speed.

### 268. A decayed-reason sweep reads every status that can decay

`--bin inapplicable` asks of each `inapplicable` row whether the tree now does what the row says it does
not; it reads `inapplicable` rows alone. Review 1768 re-read the 84 `out-of-scope` rows by hand and found
twenty-one of §13.2's still saying "a media engine, not a rendering question" while `pdf_model::media`
cites §13.2.3's subclauses, §13.2.4.3 and §13.2.7.2.2 by number and tests them — the ledger had moved only
§13.2.2 and §13.2.4.2 when that code landed (ADR 2370). A reason that a clause is not this program's can
decay under any status, so the sweep reads every status that carries such a reason, and a clause
citation under `crates/` against a row that denies the clause is a finding whatever the row says.

### 269. An advisory's named fix is checked against the yank list before it is pinned

`deny` went red on `ecb6bea0` for RUSTSEC-2026-0387 and a yank on `crypto-bigint` 0.7.5, reached through
`elliptic-curve` from `pdf-signature`. The advisory's own text said to move to 0.7.6; by the time round
1770 read it, 0.7.6, 0.7.7 and 0.7.8 had all been yanked too, and pinning the advisory's version would have
left `deny` red for a different reason (ADR 2374). Before a version is pinned to answer an advisory, the
registry's yank list is read, and the newest unyanked release that satisfies the advisory is the one taken
— in both lockfiles, the fuzz workspace's beside the root's.

### 270. `tools/batch.sh gates` is the merge's run, not a list

Round 1764's brief said the writer corpus gates were the ones "`tools/batch.sh gates` names", and the round
ran the command to read the list. It is the merge's whole gate run: it truncates the shared
`/home/AI/batch-gates.log` first, then runs every gate in order; the round ran four and stopped the fifth
after 101 s, and the merge regenerated the log (ADR 2362's slot). The list of gates is `doc/todo/02`
section 2's map and what `tools/round.sh <kind>` prints for a kind; a brief that owes a round certain gates
names them by their own commands, and a round runs them on its own export. The subcommand refusing to run
outside a merge, or a `--list`, is the instrument's side of the fix.

### 271. A re-run frame reports only the re-run's phases

Raster's compute lane sizes its first frame's edge buffer by a guess, and a frame whose count exceeds it
overflows and runs again (raster ADR 0095). The frame that lands is the second attempt, and its phase list
is the second attempt's: on HEAD the first compute window of a toolkit zoom reported 0.05–0.08 ms of
residency and no `compute pipeline compile`, while the window's wall time carried both the compile and the
first attempt (round 1760, ADR 2355 — this tree's first window reports 0.4–0.8 ms once it runs once). A
first-use cost is read from the frame's whole wall time, or from an attempt known to be the first, and
never from the phase list of the attempt that was delivered.

### 272. A sweep over notes written to one template finds the template

Round 1771 widened `--bin inapplicable` from `inapplicable` rows to `out-of-scope` rows as well (trap 268)
and found that vocabulary alone cannot separate them: every `out-of-scope` note is written to one template
that names `CLAUDE.md` and the exclusion's entry, so a term the tree also uses makes every such row a
finding and the count measures the template rather than the tree. The discriminator is the term a row
cannot fail to state — its own clause number, matched as `§` and the number with nothing after it that
continues a clause number — and a citation of it under `crates/` is what says the tree executes the clause.
On HEAD's ledger it named 31 of the 118 swept rows, ten of them among §13.2's 21 executed rows; after the
moves, none (ADR 2376). A sweep's terms are chosen from what differs between rows, never from what a
template makes them share.

### 274. A GPU figure carries the device's clock level, and arms are compared at one level

Round 1777 sampled `/sys/class/drm/card1/device/pp_dpm_sclk` every 2 ms through the turn gate's child for
`issue19802.pdf`. With nothing else on the device the Radeon 890M held its lowest level, 600 MHz, through the
whole child, and the step read about 5 ms on every tree; it rose to 1100 MHz or more only while another
process — the owner's session, which this account cannot list — kept the device 30–49 % busy, which is the
state ADR 1537's busy check refuses to judge. The "two speeds" trap 262 reads from emit+deposit are these
levels. At one level the arms separated cleanly (HEAD 2.74–3.28 ms, the fix 1.71–1.89, ecb6bea0 1.70–1.76;
ADR 2388); across levels the comparison measures the clock. A GPU-bound figure carries the sclk residency
of its run beside it, and a difference is attributed only between arms that ran at the same level. The
turn gate reads it itself: every figure prints its residency per level, a figure read wholly at the
lowest level is judged against its row's `_lowest_ms` band where the row states one, a figure whose
clock moved between levels is not judged where both bands are stated, and the render node's holders
of this account are listed beside each child (ADRs 2402, 2403).

### 275. `pp_dpm_sclk` is three lines on this APU, and the middle one is the clock of the moment

Round 1784 built the turn gate's clock sampler (ADR 2402) and found the Radeon 890M's `pp_dpm_sclk` is not a
list of fixed levels: it prints the minimum (600 MHz), a middle line that is the current clock between the
ends, and the maximum, and the star sits on whichever line the clock is at. The middle line read as low as
605 MHz and as high as 1 687 MHz across one hold, and whole gate children ran there at 4–14 % device busy —
so trap 274's "it climbs only while another process keeps it 30–49 % busy" did not hold, and three existing
step bands (EC3 p101, `images.pdf`, `issue14415.pdf`) turned out to have been taken raised. A figure is
compared by the table line it ran at together with the MHz range the sampler printed beside it; a "raised"
or "lowest" label without the range is not evidence that two arms ran at one speed.

### 277. The Radeon's lowest clock comes with the processor's load, not with a quiet device

Round 1793 held HEAD's turn gate eighteen times in one `--clock --nice 0` hold to give three step rows their
`_lowest_ms` bands (ADR 2420), each arm started once the one-minute load was under three (habit 186). Of the
231 gate children started at load under three, none ran at 600 MHz; of the 135 started at load nine or
more, 80 did. The Radeon 890M shares its power budget with the processor, so a busy processor holds the
device at its minimum and a quiet one lets it climb — the opposite of trap 274's "with nothing else on the
device it held its lowest level", and the middle line of trap 275 read from 601 MHz, not 605. A hold whose
arms start quiet therefore cannot be relied on to give lowest-level figures, and `images.pdf` and
`issue14415.pdf` got none in eighteen runs. A figure's level is read from its own sampled residency, never
inferred from the load it started at or from the device's busy share, and a raised band states the slowest
clock its runs were read at (`<figure>_raised_mhz`, ADR 2421), so a figure on the middle line just above the
minimum is not judged against a band taken at 860 MHz.

### 278. A plant is reversed by its own patch, taken against the copy before it, never against HEAD

Round 1790 planted a calibration defect (trap 13) in `crates/pdf-model/src/xfa/datasets.rs` — the
canonicalisation removed, so both new fixtures in `tests/xfa_datasets.rs` had to fail — and reversed it with
`git apply -R` of a `-U0` diff of that file. The diff had been taken against HEAD, and the round had already
edited the file, so it held all of the round's hunks beside the plant, and the reversal returned the file to
HEAD (ADR 2414). The file was restored from a copy the round had taken before planting. The tests-gates habit
"A measurement patch is applied and reversed as a patch file" holds only where the patch is the plant alone:
the plant's patch is `diff -u <copy before planting> <file>`, applied and reversed as itself, and `git diff`
against HEAD is never the record of a plant in a file the round owns.

### 280. A workspace rustdoc red with no lint in it is read under the walk's data limit first

Batch 119's merge saw tier 1's docs step (`tools/state.sh docs`, rustdoc with warnings denied over the whole
workspace) exit 101 three times with nothing but "could not document `viewer-gtk`" and "could not document
`viewer-ui`" in the step's tail, while every crate rustdoc'd alone with `-D warnings` was green. The full
output, taken by running the walk's own command line by hand, was rustdoc's allocation failure — "memory
allocation of 1567555872 bytes failed" in `html::render::search_index::SerializedSearchIndex::write_to`, under
`write_rendered_cross_crate_info` — and the walk's own sentence: stopped by the data limit after 36 s, the
tree resident at 7.13 GiB under an `RLIMIT_DATA` of 12 GiB. The cross-crate search index the workspace's
documentation shares (104 MiB on disk at that merge) is rewritten by every crate's rustdoc, and it is address
space that crosses the limit, not residency. The same command outside the walk exited 0. A docs red whose
tail names no lint is re-run as the walk's command with its whole output kept before any doc comment is
suspected, and the bound is named in the merge's words as the instrument's, never as the tree's.

Closed by ADR 2466: the docs step's walk carries a data bound of its own, `DOCS_DATA_GIB` in
`tools/state.sh`, sized from the step's measured address-space peak, and its filter keeps the walk
wrapper's own "STOPPED BY THE DATA LIMIT" sentence, so a stop by that bound reads as itself in the
step's tail. The rustdoc `-D warnings` over every crate and `--document-private-items` is unchanged.

### 281. A seed taken after the export's extraction

`tools/batch.sh export` extracts HEAD with today's time (`tar -m`) so that a rebuild over a re-export is not
handed the previous binary (trap 114), and habit 185 then seeds the export's target by reflink from the batch's
build directory. In batch 120 two rounds seeded after siblings had rebuilt that directory from their own edited
sources: those artefacts were newer than the export's extracted sources, cargo judged them fresh, and the
export's clippy compiled HEAD's crates against a sibling's mid-edit `pdf-syntax` — round 1798 (ADR 2430) saw it
as a sibling's lint, round 1804 (ADR 2442) as E0004 in HEAD's `pdf-font`. Both rebuilt after touching the
export's `.rs` and `Cargo.toml` files. A seeded export's sources are touched after the seed and before its first
build, or the seed is taken before the extraction; a build error in an export that names a crate the round did
not change is this trap until a touched rebuild says otherwise.

Closed by ADR 2467: `tools/batch.sh export --seed <build directory>` copies the seed itself and then
stamps every regular file it extracted with a time later than anything the seed holds, so the
export's first build compiles every workspace crate from the export's own sources;
`tools/conformance/tests/batch.rs` holds it with a seed whose artefacts are dated ahead of the clock.
