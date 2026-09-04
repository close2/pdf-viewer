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
it. A missing worker and a stale one look nothing alike.

**"Both gates fail loudly if the worker is missing" was this paragraph's last sentence, and it was
true of *two* gates out of eight.** `pdf-model`'s `corpus` and `oracle` check; the accessibility
census, the selection census, `text_extraction`, `fixed_documents`, `jpeg2000` and
`render-quorra`'s `corpus` did not — and the census's ratchet moved by nine structure elements
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

**It is not only a sweep, and `doc/todo/02` §5 was the instruction that got it wrong.** That section
told a round to `install` from `/home/AI/cargo-target/pdf-viewer/release/`, a *literal* path, and in
a worktree round that is a **neighbour's** build directory — so the binaries a person picks up, and
every launch measurement taken from them, are another branch's. The seven-hundred-and-twenty-sixth
session spent three rebuild-and-run cycles on it: the GTK host was rebuilt each time, installed each
time, and each time ran the main tree's binary and printed nothing for a feature that was working.
The tell is the same one as above — *nothing moves when you re-run after an edit* — and the fix is
the same: derive the directory. §5 does now, `tools/round.sh` always did, and `tools/state.sh disk`
does since the same round.
- **A before/after comparison needs a before that was compiled from the "before" tree.** `git stash`
  is forbidden here (the stack is shared), so export it — `git archive HEAD | tar -x -C <dir>` — and
  build the sweeps *inside* the export with its own `target-dir`. The export carries only tracked
  files, so symlink `doc/md`, the specification PDFs, the submodules and `corpus-cache` back in
  first; without `doc/md` three sweeps refuse outright, and without the rest `pointers` reports
  dozens of live paths as "not carried" and the comparison invents deltas the round did not cause.

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
`--release --bin pdf-viewer` — the subset resolves `num-traits`, `once_cell`, `rustix`,
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

### 27. An assertion on a substring passes for every answer that shares it

Trap 11 on the other side of the wire. A report is only as good as the condition it fires on; an
**assertion** is only as good as what its expected value *excludes*.

`pdf-syntax/tests/encryption.rs::an_unspecified_revision_is_refused_by_name` opened
`issue21579.pdf` and asserted the refusal's sentence contained `"/R 5"`. Delete §7.6.4.2's refusal
entirely — accept revision 5, which Table 21 says "[s]hall not be used" — and the same document is
declined a few lines later by §7.6.4.1's crypt-filter pairing, whose sentence begins *"/R 5 with a
crypt filter method"*. The substring is there, the test is green, and the clause the test is named
after is not implemented at all. Two ledger rows rested on it.

**This paragraph used to add "and states no algorithm for" to Table 21's sentence, and the
eight-hundred-and-eighty-seventh session implemented the revision** (ADR 0820), which changes
nothing about the trap and is worth one line all the same: the counterfactual above — *accept
revision 5 and watch the test stay green* — is now the tree's actual behaviour, and the test that
survives asserts on `/R 7`. A trap keeps its incident; it does not keep the incident's claims about
the standard.

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

### 29. A bound lifted in a scratch build is lifted only where the code reads the constant

"Lift the bound sixteenfold and see which documents still reach it" is this project's standard
experiment on a budget, and ADR 0271 ran it on `MAX_FORM_DEPTH` over 65 944 documents: all four
witnesses reached 256, so all four were cycles, so the bound was the attack it exists for. Two more
rounds repeated it over two more corpora and the eleven-document claim went into the constant's own
comment. Twenty-five of the twenty-seven were finite — a tiling cell was run at `MAX_FORM_DEPTH - 1`,
so lifting the constant lifted the cell's starting point with it, and a cell holding two levels of
forms reported the bound at sixteen, at 256, and at any value a scratch build could name (ADR
0793).

The experiment had no control. Trap 13's rule for a sweep — run it against the defect before
believing it — has a mirror for a lifting: **run it against a document known to be finite and
deep, which must stop**, before believing that whatever still reaches the lifted bound is a cycle.
And read every site that *derives* a number from the constant, because those move with it: a
`grep` for the name finds them in a second and the eight-hundred-and-seventy-first session, which
found the two nestings, did not look.

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

## Things worth knowing

- **The sandbox is a flag and the default is the safe one.** `--no-sandbox` trades panic
  containment and a memory ceiling, not memory safety. There is deliberately no path that falls
  back to in-process decoding when the worker fails to start.
- **Debug builds are ~15× slower here**, and it changes what a test can assert: the corpus gate is
  2 s in release and minutes in debug. Run timing assertions in release and say so.

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
