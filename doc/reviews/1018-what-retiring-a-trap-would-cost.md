# 1018 — What retiring a trap would cost

Status: **review** — read-only over `doc/traps/`, `crates/` and `tools/`, commissioned by the project
owner in session 1018 alongside the trap index (ADR 1036). Proposals only; **nothing is deleted by
this round**, on ADR 1005's precedent that a review proposes and the owner decides.
Date: 2026-09-13, with five sibling rounds live in the same worktree.
Record: `doc/history/1018-what-a-round-reads-before-it-works.md`.

`§N` is a clause of ISO 32000-2 and nothing else; other standards are written out in full.
Quotation marks mean verbatim.

---

## The counts

Sixteen traps were handed to this round as the genuinely cold set — at least 74 sessions old **and**
under 4 citations per 100 sessions since first citation: **4, 6, 7, 14, 17, 18, 19, 20, 21, 22, 23,
26, 29, 30, 31, 36**.

| recommendation | traps | count |
|---|---|---:|
| **RETIRE** — a program can make the mistake impossible, on one condition stated below | 7 | **1** |
| **MERGE** into a heavily-cited trap | 4 → 8, 29 → 13 | **2** |
| **DEMOTE** to `doc/habits/` | 36 → `measuring.md` | **1** |
| **KEEP** — rare because the position is rare, and expensive if sprung | 6, 14, 17, 18, 19, 20, 21, 22, 23, 26, 30, 31 | **12** |

**Not one of the sixteen is retirable today without work.** That is the finding. The only RETIRE on
the list needs a lint enabled and three existing violations fixed first, and the evaluation turned
those three up as a live defect (below). Every other cold trap either has no program that could hold
it, or has a program that holds the *instance* and not the shape.

**The age-normalisation the owner ran is the reason this list is not longer.** Raw counts would have
put trap 37 (5 citations) and trap 39 (12) on the block; normalised, they rate 31.2 and 26.7 per 100
sessions — trap 37 is level with trap 1's 31.6. A raw-count cull would have cut two top-tier traps.
Rarity is not a verdict, and three of the four recommendations here keep the incident intact.

---

## RETIRE

### Trap 7 — `#[expect]`, never `#[allow]`

11 citations in 555 sessions on the owner's age-normalised count, and the extreme of the cold set —
and it is the one trap whose whole content is a rule a compiler can hold.

**The mechanism exists and is verified present in this toolchain**: `clippy::allow-attributes`, a
restriction lint, confirmed by `clippy-driver -W help` under clippy 0.1.97 —

> `#[allow]` will not trigger if a warning isn't found. `#[expect]` triggers if there are no
> warnings.

Set to `warn` in `[workspace.lints.clippy]`, it becomes a build failure under `doc/todo/02` §2's
`RUSTFLAGS="-D warnings"` line, which is tier 1 and runs in every round. That is the criterion met.

**It cannot be retired today, and the reason is a defect this evaluation found.** The lint is *not*
in `Cargo.toml`'s lint table, and the tree currently carries **three `#[allow(clippy::…)]`
attributes the trap forbids**, all committed:

```text
crates/pdf-transform/src/archive/mod.rs:435   #[allow(clippy::too_many_arguments)]
crates/pdf-transform/src/archive/mod.rs:646   #[allow(clippy::too_many_arguments)]
crates/pdf-transform/src/archive/remedies.rs:184  #[allow(clippy::too_many_arguments, reason = "…")]
```

The third carries a `reason`, so its author knew the convention and reached for the wrong attribute
anyway — which is exactly the evidence that eleven citations of a prose rule is not enough. The
other ten `#[allow(` matches in the tree are `#[allow(unsafe_code)]` at the three foreign-function
boundaries and the test files that assert on that literal string; enabling the lint has to answer
those deliberately (they are pinned by `crates/*/tests/unsafe_position.rs`, which match on the text
`#[allow(unsafe_code)]` and would have to move with them).

**Recommendation: enable `clippy::allow-attributes`, convert the three, decide the `unsafe_code`
five, and then delete trap 7's prose** — the only entry on this list where deleting prose loses
nothing, because the rule survives as a compiler error. Cost: under a round. Until that is done the
trap stays, because a rule enforced by nothing is not enforced by a file that says it.

---

## MERGE

### Trap 4 — Test against real documents, not hand-written fragments → **trap 8**

Trap 4's own text names trap 8 as its converse, and trap 8's opening line names trap 4 as its
mirror; they already live in the same group file, seven headings apart. Trap 8 is cited 168 times,
trap 4 eight. A round that reads trap 8 — *a corpus finds what documents contain, not what the
specification says* — is reading half of one rule, and the half it is not reading is the half that
says the hand-built fixture is not the tree either.

**The merged trap keeps both incidents**: trap 4's cross-reference streams failing with a misleading
`/Root is not a dictionary` because the decode was "the caller's responsibility" and then was not,
and trap 8's four (the hand-assembled ICC profile turning white into pure green, the two rules
measured unreachable by breaking them, the §7.6.2 grep whose sets were not what it thought, and the
deleted-object pair). Nothing in trap 4 is dropped; it becomes trap 8's first section, keeping the
number **4** as a citable sub-heading exactly as 10a, 10b, 12a, 12b and 12c do.

Cost of not merging: small — trap 4 is short. The gain is that the pair is read together, which is
the only way either half is true.

### Trap 29 — A bound lifted in a scratch build → **trap 13**

Trap 29's own text makes the argument: *"Trap 13's rule for a sweep — run it against the defect
before believing it — has a mirror for a lifting."* Trap 13 is the most-cited trap in the project
(288 citations); trap 29 has four. A lifting experiment is a sweep whose instrument is a constant,
and its calibration is trap 13's calibration with the sign reversed — run it against a document
known to be **finite and deep**, which must stop.

**The merged trap keeps both incidents**: ADR 0271's `MAX_FORM_DEPTH` over 65 944 documents where
all four witnesses reached 256, and ADR 0793's twenty-five of twenty-seven that were finite because
a tiling cell was started at `MAX_FORM_DEPTH - 1` and moved with the constant. It also keeps trap
29's second rule, which trap 13 does not have — **read every site that *derives* a number from the
constant, because those move with it** — as a bullet under the merged heading.

---

## DEMOTE

### Trap 36 — A neighbour takes half a figure, and `/proc/self` is the wrong thread → **`doc/habits/measuring.md`**

Two citations. The position that springs it is *taking a timing figure on a machine other rounds are
using*, which on this machine is every timing figure there is — so two citations is not rarity, it
is **the trap being in a file the round that needed it did not open**. A measuring round opens
`doc/habits/measuring.md` (HANDOVER's table says so, and so does `tools/round.sh`); trap 36 is in
`doc/traps/instruments-and-reports.md` under a heading about gates.

It is also, unlike its neighbours there, method rather than a defect in a program: *the wait
explains the tail, the sharing explains the level*, and *ask `/proc/thread-self`, not `/proc/self`*
are two sentences about how to take a number, which is what that habit file is. `measuring.md`
already carries "A/B in one sitting" and "attribute by removing the suspect"; this belongs beside
them.

**The move keeps everything**: ADR 0916's two measurements (eight pinned spinners raising a
one-millisecond figure by 43% and a fixed-work probe by 74% with the kernel's wait counter reading
exactly zero in all twenty samples; the one excursion of fifteen reading 3.947 ms against a wait of
2.825), and the sentence about every other per-task file under `/proc/self/`. The number **36** is
kept as the sub-heading, because ADRs cite it.

Note against this: `CLAUDE.md` and HANDOVER both say a lesson lives in exactly one place — a trap if
it changes how you write code, a habit if it changes how you work. Trap 36 changes how you *measure*,
which is the habit side of that line. Traps 34 and 35 are the same family and are **not** on the cold
list, so they are not proposed for the move; if the owner takes 36 across, whether 34 and 35 follow
is a separate question with its own evidence.

---

## KEEP

Twelve, each with what it would cost if sprung and why no program holds it.

### Trap 6 — Colour: one conversion, and the specification often has no answer

`crates/pdf-model/tests/colour_paths.rs` exists and drives one colour through the `k` operator,
`scn` in a `DeviceCMYK` space and a CMYK image's samples, demanding they agree. **It does not retire
the trap, and the reason is trap 25's shape**: its routes are a *hand-written* list of five test
functions, so a fourth path added tomorrow is outside the population and the test stays green. The
trap's rule — `ColourSpace::to_rgb` is the only place a colour becomes RGB, `colour::xyz_d50_to_srgb`
the only place an XYZ becomes a pixel — is what the test cannot enforce. Cost if sprung: two colours
that both look plausible and are not the same colour, with which one you get depending on how the
producer happened to write the file, and nothing about a rendered page revealing it. It already
recurred once, one level down, in a nine-constant matrix.

### Trap 14 — A target that *is* the region a clause names cannot tell you whether you applied it

No program holds this and no program can: the trap is about what a gate's **extent** makes
indistinguishable, which is a property of every gate in the tree at once. Cost if sprung, measured:
§14.11.2.1's crop-box clip was missing for the whole life of the tree, invisible to the corpus, the
oracle and the quorra comparison because all three rasterise a page-sized target, and the census
that followed found **3690 of 66 887 first pages** marking outside their boundary. The position is
rare — a clause that names a region — and the trap's second rule is three minutes of work
(rasterise something bigger once, by hand).

### Trap 17 — A toolkit's widget list is a catalogue, not a statement of what it can do

Rare because the position is rare: it springs only when a round is about to write that a host
*cannot* obey a clause. Cost if sprung, measured twice on the same host: Table 233 bit 19's editable
combo box was ranked last for thirty-nine sessions on a block every sentence of which was true, and
the answer was a `GtkEntry` beside a `GtkMenuButton` over a `GtkListBox`; ADR 0508 was Table 234's
`/TI` in the same host. **Both times the capability was one composition away and the block named a
symbol** — a deferred feature apiece. Borderline demote (it is about how a claim is written), and
kept as a trap because what it describes is being fooled by a catalogue rather than a practice to
adopt.

### Trap 18 — A limit a process is under can destroy the channel it reports through

`crates/viewer-confined/tests/confined.rs::a_confined_worker_cannot_write_a_diagnostic_to_a_file`
exists and pins the instance on a single write — verified present. It holds `RLIMIT_FSIZE` against
an inherited standard error and **nothing else**: the trap's general rule names `RLIMIT_NOFILE` and
a seccomp filter as the same shape, and no test reaches those. Cost if sprung, measured: the same
document, the same worker and the same defect reported as `killed by signal 6` with the worker's own
`memory allocation of 1899996152 bytes failed` down a pipe, and as `killed by signal 25` with
**nothing at all** down a file — and the round that first met it read `SIGXFSZ` as a defect in the
code it was measuring.

### Trap 19 — A widget the *document* placed can decide how big the window is

No program; the answer is structural in both hosts (a `GtkOverlay` child that GTK does not measure,
a `PageArea` with no layout at all) and neither arrangement is asserted anywhere. Cost if sprung:
nothing looks like a defect — the windows appear, the page draws, no gate can see it — and the only
tell is that one `Resize` line in a trace has become nine, the page area walking 509 → 1229 device
pixels over a geometric series that converged only because each step halved.

### Trap 20 — `Rendered::Failed` marks a page as answered

Two mechanisms exist and neither closes it.
`crates/viewer-core/tests/headless.rs::a_refusal_is_final_for_this_view_and_a_token_never_answered_is_not_re_asked`
is present and asserts both halves — **of the core's behaviour**, not of a host's choice — and
`viewer_host::drawing::Finished::outcome` being `Option<Rendered>` makes the rule visible at the
call site. A host can still write `Some(Rendered::Failed)` for a draw it abandoned itself, and that
is the whole defect. Cost if sprung: the page is frozen for the rest of the view, the person gets
`doc/todo/37`'s stand-in permanently, and the status line blames the document for a decision the
host made about its own thread. ADR 0650 §6 predicted the opposite **in writing**, which is the
strongest possible argument for keeping the prose.

### Trap 21 — A toolkit's main loop cannot dispatch your poll while it is inside its own frame

No program. Cost if sprung, measured: page one drew in **3.3 ms** and the window waited **61.5 ms**
for it, twenty runs an arm with no overlap between the ranges — and a round reading only the first
number sees a fast rasteriser. `CLAUDE.md` principle 2 makes that number a first-class requirement,
so a trap that stops it being misread is load-bearing however rarely it is cited. Its second half is
the two-host reading that located it, which no instrument produces on its own.

### Trap 22 — A shared key table is only as level as the narrowest path a key takes to reach it

No program, by construction: the trap's own last paragraph says the only instrument is
`doc/environment.md`'s `Xvfb` recipe, a real window, a real key, and looking. Every existing test
asks whether a host *can* obey the table; none asks whether a press gets there. Cost if sprung,
measured: `viewer-qt` swallowed Escape entirely from the round the table was written (ADR 0526) to
session 795 — §12.4.2's "Escape clears the selection", one of the three disagreements the table was
created to settle, never reaching the table in that host — while the crate's tests, an exhaustive
match in three hosts and the shared documentation all said it did.

### Trap 23 — `--all` and `--workspace` are scoped to a workspace, not to the tree

**Half of this one is now held by a program, and it fires**: `tools/conformance/tests/workspaces.rs`
derives the population from cargo rather than listing it, so a crate kept out of the workspace fails
on the day it is added — run in this session, 2 passed. The trap says so itself and says what is
left: *"It does not hold the module-graph half closed, and nothing does."* A `.rs` file no `mod`
declares is unformatted however the workspaces are arranged, and the only thing that has ever
checked is a measurement somebody took by hand (`cargo fmt --all -- --emit stdout` against
`git ls-files '*.rs'`). Cost if sprung: two rustfmt diffs under a green formatting gate for the life
of `fuzz/`, and thirty-three clippy findings — five of them arithmetic in a target's own counters
under a profile that keeps overflow checks on — because a lint level also stops at the boundary.
**Recommendation inside the KEEP: narrow the trap to the module-graph half and cite `workspaces.rs`
for the other**, so the prose covers only what nothing holds.

### Trap 26 — The worst tile is measured on a fixed grid

`raster_compare::DEFAULT_TILE`'s doc comment carries the paragraph, and
`the_same_difference_reads_half_as_much_when_it_straddles_the_tile_grid` is present in
`crates/raster-compare/src/lib.rs` and pins the halving in arithmetic. That test proves the
*phenomenon*; it cannot stop a round ranking two pages by the figure, which is the mistake. Cost if
sprung, measured: the same glyph and the same difference read 62.57 on one page and 35.32 on
another — a factor of 1.77 — and the reading that was in the tree instead blamed the references'
consensus, which measurement showed sat **closer** on the pages in question, so the bound went the
other way and our own number carried all of it. A whole diagnosis pointed at the wrong renderer.

### Trap 30 — A sink keyed by name hands its outputs back in the order they were *opened*

The prescribed helper exists — `crates/pdf-transform/tests/split.rs`'s `piece()` looks an output up
by name and panics naming every output there was — so the instance is fixed. Nothing generalises it:
the trap's rule is about any report, listing or map whose producer is parallel, and `Report`'s
`outputs` *are* in plan order while `MemorySinks`' are not, a difference no type states out loud.
Cost if sprung: two rounds of a gate that passed by scheduling luck, then
`assert_eq!(Pages::new(&first).len(), 2)` failing with `left: 1, right: 2` — a gate reporting on the
scheduler under a name that promised something about the writer, which is the most expensive kind of
red there is.

### Trap 31 — A fallible filesystem call is not a *safe* filesystem call inside the confinement

`crates/pdf-vfs/tests/confined.rs` re-executes itself under the confinement and checks that a
forbidden system call kills and that a font looked for on the machine does not — present, and it is
the trap's own calibration. It holds the two calls it names; it cannot see the next crate linked
into a confined worker. Cost if sprung, measured: `SECCOMP_RET_KILL_PROCESS` does not return an
`Err`, so the careful `let Ok(entries) = read_dir(dir) else { return; }` never reaches its `else`,
the `openat` ends the process, and **the mount loses the whole generation — the viewer and the
page**. Four of the first sixty documents the read side's corpus walk touched did this. The
population it names is not "code that unwraps" but **code that opens**, which no lint expresses.

---

## One thing this evaluation is not

It is not a ranking by how good a trap is. Trap 22 has two citations and describes a defect that
lived in a shipped host from the round its key table was written until session 795; trap 7 has
eleven citations and is violated three times in the tree today. **Citation frequency measures how
often a round was in a position to spring the trap, not how much it is worth** — which is the
reason the index built by this round (`doc/traps/README.md`) is organised by that position rather
than by the number.
