# 813 — Four merges, and four silences that were not evidence

Date: 2026-08-28. A merge round: `main` at `120951e7` (808's result), four parallel rounds
integrated in round order, the whole of §2 run on the merged tree, §5's eight artefacts rebuilt and
installed, §4's twenty sweeps taken against a pre-merge baseline, and the four worktrees closed.
No feature work, and this round decided nothing that needs an ADR.

`main` is far ahead of `origin` and was **not** pushed. `doc/rfc/` was not touched.

## The merges

| | branch | tip | merge commit | files |
|---|---|---|---|---|
| 809 | `round-809` | `1332b481` | `6d00714f` | 3 documents, +978 lines, no source |
| 810 | `round-810` | `29f7edbb` | `6f8f0733` | `fuzz/` under the tree's lints, §2's fuzz line, `workspaces.rs`, `tools/worktree.sh`, `tools/fuzz.sh`, `tools/state.sh` |
| 811 | `round-811` | `a0f48c0b` | `df49cec1` | `structure.rs`, four ledger rows, `doc/errata-read.md`, `doc/todo/01`, `doc/ledger-and-claims.md` |
| 812 | `round-812` | `53613b8a` (3 commits) | `7619c4ab` | `filter.rs`, two test files, two ledger rows, `doc/todo/18` deleted |

**Four clean merges, one auto-merge, no conflict resolved by hand.** The only file two branches
touched is `doc/conformance/ledger.toml` — 811's four §14.8.5.x rows against 812's §7.4.1 and
§7.4.4.1 — and `ort` merged them without a hunk in common, which is what the briefing predicted.
`doc/todo/README.md` also auto-merged: 809 adds row 56, 812 removes row 18.

**809 branched from `f8ccf50c`**, an intermediate commit of merge round 808, rather than from
`120951e7`. That is expected and it was not rebased; the merge is a fast three-way against a base
that is an ancestor of `main` either way.

**Gitlinks.** `git ls-files -s doc/pdf.js doc/arlington-pdf-model` read `160000` before every
commit, and `conformance::every_declared_submodule_is_still_tracked_as_one` passes on the merged
tree. Every stage in this round was by explicit path; no `git add -A`, no `-u`, no `stash`.

## §2 changed under this round, and the merged tree's §2 is what ran

Round 810 replaced §2's `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml
--bins` with `RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets`.
The sequence run here was read out of the **merged** `doc/todo/02-every-round.md`, line by line,
after the fourth merge — not from the copy this round started with. Three things follow and each
was checked rather than assumed:

- **The new fuzz clippy line is clean on the merged tree.** It rechecks `pdf-syntax`, `pdf-font`,
  `pdf-model`, `viewer-core`, `viewer-confined` and then `pdf-viewer-fuzz` itself, exit 0. This
  matters more than a merge usually makes it: 812 changed `pdf-syntax/src/filter.rs`, which is a
  crate three of `fuzz/`'s targets link, and until 810 landed nothing in §2 linted them.
- **`tools/conformance/tests/workspaces.rs` passes with its two new arms**, and it is designed to
  fail when a root lacks a required line, so this is a positive result rather than a silence.
  `fuzz/Cargo.toml`'s `[workspace.lints.rust]` and `[workspace.lints.clippy]` were also compared by
  hand against the root workspace's: identical, entry for entry, all fifteen levels.
- **The old `check` line is gone from the tree**, so no round can run the weaker gate by copying an
  older invocation.

## The new ledger status

811 wrote the ledger's **first `silent` row** (§14.8.5.5). `Status::Silent` itself is older — it is
in `tools/conformance/src/ledger.rs` at `120951e7` and was carrying a population of zero — so what
this round had to check is that the *gate* accepts a row that finally uses it. It does:
`cargo test -p conformance` prints the status breakdown with `silent 1` in it and
`the_ledger_agrees_with_the_standard_and_with_the_tree` passes. **0 rows are `unreviewed`**, in
every one of clauses 7 through 14.

`cargo run --release -p conformance --bin ledger` was then run as the briefing's caution 5 asks —
875 rows, 0 new — and **it produced no diff at all**. The formatter's output is already what the
merge committed, so there was nothing of its own to commit.

## Pointer absences

`--bin pointers` reads **9212** path pointers on the merged tree against 9134 at the baseline:
live 5189 → 5226, unrooted 3097 → 3119, a form 200 → 202, not carried 527 → 540, **absent 98 →
102**. Symbol pointers stay 162 with 13 undefined.

**All four new absences are `doc/todo/18` and nothing else** — the baseline report names that path
zero times and the merged one names it four:

| where | line |
|---|---|
| `doc/adr/0730-what-incomplete-is-made-of.md` | 109 |
| `doc/adr/0744-a-flush-is-not-a-truncation.md` | 3, 60, 77 |

That is the intended convention rather than a defect. An ADR is a dated record and is not edited to
follow a file that moved under it (ADR 0232 §2); 0744 names the item it closed, which is what a
closing decision is for. `doc/todo/README.md`'s rule that a deleted item's number is not reused is
the other half of it.

## ADR verification

`main` carried through **0739** pre-merge. The batch brings **0742** (810), **0743** (811) and
**0744** (812); 809 is a research round and deliberately took none.

**0740 and 0741 are unused, and that was verified rather than assumed.** No file exists at either
number, and `grep` over every `*.md`, `*.rs`, `*.toml` and `*.sh` in the tree (excluding `doc/md/`)
finds not one reference to `ADR 0740`, `ADR 0741`, `adr/0740` or `adr/0741`. 0740 was never claimed
by any round — 808's own record already says "nothing at 0740 or above". 0741 was reserved for 809
by its briefing and 809 declined it in writing, on the ground that nothing was decided and that the
decision belongs to whichever round implements the owner's ruling. Gaps are this tree's existing
convention, confirmed by merge round 803. `tools/state.sh counts` reads 632 ADR files against a
highest number of 0744.

## Gates (full §2 sequence on merged `main`, quiet machine, nothing beside it)

Nineteen lines, **every one exit 0**. The machine carried nothing else: one sleeping 15-day-old
`quorra_gpu` process at 0.0% CPU and `sccache`'s server, and the sweeps were run before and after
the sequence rather than beside it.

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| clippy, `-D warnings`, `--workspace --all-targets` | clean, exit 0 — the only `warning:` lines are gcc's on the `cxx-qt` bridge and cargo's standing `proc-macro-error2` future-incompat note |
| `cargo nextest run --workspace` | **2786 tests run, 2786 passed, 18 skipped** |
| `cargo test --workspace --doc` | ok, 24 suites |
| `cargo fmt --manifest-path fuzz/Cargo.toml --check` | clean |
| **`RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets`** (810's replacement line) | clean, exit 0 |
| `cargo build --profile gates -p pdf-sandbox --bins` | ok (trap 10) |
| pdf-model corpus | ok — 974 documents in 3.4 s, 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, **63 incomplete**, 0 slow; composition the file 53, neither one 9, this reader 1 |
| `pdfref-hayro` build | ok |
| oracle | ok — 1945 pages in 52.8 s (1842 complete, 103 incomplete), **983 agree, 61 contradicted, 836 ambiguous**, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render; 6707 of 6707 reference renders from the cache, 0 produced |
| text_extraction (three gates) | ok — pdftotext 99.3% (24014/24193), PDFBox 99.8% (14257/14281) in both orders, position verdict 10971/11163 (98.28%), **487 of 508** documents fully in bounds |
| selection_census | ok — readback differs on 0 of 966, caret 2094 offsets over 459 fields, drag 1000/1011 (98.91%, printed not ratcheted), panicked 0 |
| accessibility_census | ok — 102853 elements, 57116 a caret can move through, 876 of 876 untagged pages answering the honest empty tree, 0 disagreeing lines, panicked 0; no ratchet moved |
| dates, xmp, jpeg2000 | ok |
| render-quorra corpus | ok — 957 pages in 26.6 s, **932 agree, 22 differ, 3 refused, 17 not comparable**; median page 1.90× the CPU backend |
| fixed_documents | **41 checked, 0 absent, 41 rows** |
| `cargo test -p conformance` | **208 passed** — 11777 citations, 875 subclauses, **0 `unreviewed`** in any clause family, 1107 quotations all verbatim, ledger prose naming 2911 clauses and 282 tables, 251 distinct tables cited by the tree |

**The nextest count is the batch's exact arithmetic**: 808's 2782, plus 810's **net zero** (it
renames `every_workspace_in_the_tree_is_formatted_and_compiled_by_the_sequence` to
`…_formatted_compiled_and_linted_…`, which the diff shows as one added and one removed), plus 811's
one (`an_artifact_owned_bounding_box_is_the_same_rectangle_as_a_layout_owned_one`) and 812's three.
2782 + 0 + 1 + 3 = **2786**, and 809 adds none because it is documents only.

**Every verdict landed where its round left it**, and the two that moved are 812's and were
predicted by it. The corpus gate's incomplete population is **63**, three fewer than 808's 66, and
the mechanism *one of §7.8.2's other content streams, drawn as far as its damage* is gone from the
printed composition entirely (`the file` 56 → 53). The text line's denominator rose with it:
24193 words against 808's 23013, with the numerator up by the same 1180, because
`comments.pdf`, `highlights.pdf` and `issue3885.pdf` stopped being *incomplete and not gated* and
every word of them matched. The oracle, quorra, `fixed_documents`, both censuses and the position
verdict are identical to 808's figures to the unit.

**`doc/todo/00`'s step 7 is not owed by this merge.** The one change in the batch that can move a
pixel is 812's, and it moved the three it measured; nothing in the merged tree draws differently
from the tree 812 gated.

## §5 — the eight artefacts

Rebuilt in `--release` and installed. The build directory was **asked for, not written down**:
`cargo metadata --no-deps --format-version 1 | jq -r .target_directory` run with the shell in
`/home/cl/projects/pdf-viewer` printed `/home/AI/cargo-target/pdf-viewer`, which is trap 15's shape
— the value follows the working directory, and a literal path would have installed a neighbour's
binary. One invocation for the seven binaries (2 m 24 s), a second for `viewer-ffi`'s `cdylib`
(44 s), `install -Dm755` for all eight. `tools/state.sh binaries` shows all eight at this round's
timestamp; `target/safedocs` is older and is not one of them.

`tools/round.sh` will report `target/pdf-viewer is older than HEAD` afterwards and be right without
anything being wrong: the commit after the install is this file.

## §4 — the sweeps, against a pre-merge baseline

**The before-half was taken in a checkout of its own.** `git worktree add --detach` at `120951e7`
under `/home/AI/pdfv-baseline-813`, with `doc/md`, `doc/*.pdf`, the two submodules, the four
corpora and `corpus-cache` symlinked in **the way `tools/worktree.sh` does it** — 809's record is
why that list is followed rather than improvised, since a baseline missing the `doc/*.pdf` links
moves a hundred pointers on its own and the delta reads as the change's. Its own `target-dir` in a
per-worktree `.cargo/config.toml`, never an exported `CARGO_TARGET_DIR`. Nothing in the working
tree was touched, so nothing could be restored away; the checkout and its build directory were
removed together afterwards.

Twenty sweeps run on both sides and diffed. **Two are byte-identical** — `callers` and `unpriced`.
**Five more differ only in line numbers** — `blockers`, `entries`, `overstated`, `capabilities`
(summary line identical) and errata `check`, all of them following 811's insertions in
`structure.rs` and 812's fourteen added lines in `oracle.rs`. Errata `moved` is the same modulo
line numbers, at 15 of 2865 annotations both sides. Every remaining delta is accounted:

| sweep | delta | attribution |
|---|---|---|
| `counts` | 8930 → 9026 sentences, 450 → 454 attributed counts, **all four** in the *attributed to a clause with no rows below it* bucket (242 → 246); agreeing stays 150, *no such way* stays 58, double-counts stay 4 | the batch's new documents; the sweep's benign class |
| `tables` | 7000 → 7053 sentences, 2596 → 2620 attributed key citations, **all twenty-four agreeing** (2428 → 2452); absent stays 101, denials stay 6, no-such-table stays 0 | 809's todo, 811's ledger notes, the three ADRs and four history files |
| `quotations` | 6766 → 6841 quotations in 1098 → 1105 documents, verbatim 2825 → 2847; **diverging still 38**. In the ledger 1983 → 1990 with verbatim 1518 → 1524 and **diverging still 2** | the same documents; no new misquotation, and 809's quotations of ISO 21757-1 and NVD correctly match nothing in ISO 32000-2 |
| `inapplicable` | **69 → 67 rows**, 315 → 299 terms, 257 → 244 named, confirmed claims 58 → 55 | 811 alone: §14.8.5.5 → `silent` and §14.8.5.8 → `partial`, both off `inapplicable` |
| `owed` | **222 → 223 `partial` rows**, 3974 → 4009 terms, debts named by no source 182 → 183 over 112 → 113 rows; **110 clean rows unchanged** | 811's §14.8.5.8 |
| `overtaken` | **46 → 45**, against 629 → 632 decision records | 812 rewriting `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE`'s note around its own finding and citing ADR 0744 in it — the sweep's own stated rule working as designed |
| `pointers` | 9134 → 9212, **absent 98 → 102** | the batch's new documents; the four absences are `doc/todo/18` and are named above |
| `parts` | two line numbers, and one more cardinal in the *dated record* bucket (332 → 333); **agreeing stays 39**, and the workspace still states 3 backends, 25 crates, 3 hosts, 6 submodules, 2 workers | no part was added to this tree; 810's trap 24 lengthening the handover's index row and §2 gaining lines |
| `unread` | **one new hit**: §14.8.5.8 claims `/Type` and `/Subtype` unread while `structure.rs`, `loading.rs` and `absence_audit.rs` name the words. 68 → 69 rows, 179 → 181 keys; **confirmed claims unchanged at 41** | 811's new `partial` row, and it is the sweep's documented noise shape — see below |
| `retired` | `truncated` 269 → 273 mentions with **corrections still 14**; `Table 385` 2 → 23 with 2 corrections; `silent is zero` 0 both sides | 812 and 811 respectively; nothing retired is still being quoted as current |
| `quoted` | line numbers only — 237 figures read, 123 confirmed, 101 contradicted, **unchanged** | 812's insertions above the notes |
| errata `applied` | 60638 → 61076 places read, 952 → 965 naming an erratum; **183 dropped `#NNN` tokens unchanged** | 811's fifteenth use of the errata rule |

### The one delta worth reading rather than accounting

**`unread`'s new hit on §14.8.5.8 is the sweep being right about a word and wrong about an entry**,
which is the shape §8.9.5.1's note already records for `/OC`. 811 moved that row to `partial` and
said in as many words that Table 385's `/Type` and `/Subtype` are *not* read; the sweep then finds
both names in the row's own `code` list, because `/Type` and `/Subtype` are the most shared keys in
the standard and `structure.rs` reads Table 363's `/Subtype` for the marked-content form of the
same information. The row already explains the distinction — what it owes is Table 385's cells on
an `Artifact` *structure element*, not the property list's — so the hit is a reading list that the
row answers, and no change is owed. It is recorded here because a later round finding it should not
have to re-derive that.

### This file put through the map

A history file is a documents-only change, so `cargo test -p conformance` ran again after the last
edit — **208 passed**, 11777 citations, 1107 quotations all verbatim, 0 `unreviewed` — together
with `--bin quotations` and `--bin pointers`, which the map names for a change that moves a
document or a pointer.

`quotations` reads 6844 in 1106 documents against 6841 in 1105 before this file, with **verbatim
unchanged at 2847 and diverging unchanged at 38**. `pointers` reads **9212, identical in every
column** — and the reason is worth writing down once, because two rounds have now had to derive it:
**`doc/history/` is not in the pointer sweep's population at all.** `--bin pointers` names zero
paths under it, so a history file can neither add an absent pointer nor retire one, and the four
`doc/todo/18` absences above stay exactly four however many session records go on citing the item
812 closed.

## Worktrees closed

`tools/worktree.sh close 809 810 811 812`, using the **merged** script — 810 changed it, and the
change is the `linked` list gaining `fuzz/corpus` and `fuzz/artifacts`, which is `open`'s half
rather than `close`'s. **Both halves went for all four, verified from evidence rather than from the
script's own message**: `ls` finds no `.claude/worktrees/*` and no `/home/AI/cargo-target/pdfv-r*`,
`git branch --list 'round-*'` is empty, and `tools/worktree.sh list` names only `main`. The
baseline checkout and its 272 MB build directory went the same way. The four build directories
`list` sized at 19, 24, 22 and 24 GB are gone with their checkouts, which is the pairing
`doc/environment.md` records 425 GB of accumulated orphans for.

`tools/state.sh counts` reads **fuzz targets unseeded here: 0** afterwards — 810's new line, and
the answer the main tree gives now that all fifteen corpora are where `open` will link them from.

## What the batch is about, taken together

Four rounds that look unrelated, and the thread is one sentence: **a silence is not evidence, and
each of these four found an instrument or an argument that had been treating one as though it
were.**

- **809 — the owner's question, answered with a todo and one finding nobody was looking for.**
  Asked whether a memory-safe ECMAScript library now exists, the round says yes with a
  qualification it puts at the top so no later round can read past it: Boa passes 95.21% of test262
  against QuickJS's 82.13%, but no engine forbids `unsafe` and none has a memory or wall-clock
  budget of its own — which this tree already supplies from the kernel, and an `RLIMIT_AS` is
  strictly stronger than an engine's accounting of its own fuel. The larger finding is the one the
  commission did not ask for: **the host object model the exclusion assumed the standard left
  undefined is defined, by ISO 21757-1:2020, a normative reference of ISO 32000-2.** That is the
  third recorded decay of "the specification defines nothing here", after `DeviceCMYK` and the
  transfer functions, and it took a `grep` for `21757`. The round wrote no ADR, no RFC, no
  `CLAUDE.md` edit and no dependency, because an exclusion is amended by argument and a round
  editing the sentence it is arguing about would be the attrition the rule forbids.
- **810 — a fuzzing gate that had never linted and, in a worktree, never had a corpus.** Two holes
  ADR 0739 left in writing, and they are one shape twice: `fuzz/` was outside the tree's lint
  levels (33 findings, five of them arithmetic overflow in a target's *own* counters under
  `overflow-checks = true`, each an abort waiting to be filed as a crash in the parser under test),
  and a fuzz run's exit status says nothing about whether it fuzzed. The measurement is the
  argument: unseeded, the `page` target reaches cov 103 and ft 182 — **the same two figures the
  `document` target reaches, to the unit** — which means it never once entered
  `pdf_model::interpret`, the entire reason it exists. And the corpus was empty in every parallel
  round because `fuzz/corpus` is gitignored and `tools/worktree.sh` did not link it. A round in a
  worktree fuzzed the recovery scanner, reported it as having fuzzed the interpreter, and exited 0.
- **811 — the ranking unit was exhausted, not the population.** The errata rule's fifteenth use
  found both row rankings flat at a head of two, with thirty-nine rows tied there; ADR 0653's
  tie-break was written for three. The temptation is to retire the rule, and the decay curve is
  what refuses it: the base population has run 133, 111, 104, 99, 85, 73, 71, **61**, six or seven
  a use, with a fifth of the collection still unread. So what collapsed is the *row* count's
  resolution, and the recipe gains a **step 5** — rank the same annotations by issue, which is the
  unit step 3 already said to read in. By issue the head is #346 at six annotations, and it reached
  §14.8.5.8's row, which had **deleted an entry Table 385 states** because the entry's cell sits
  under the next clause's heading in `doc/md/`'s page-break — a conversion artefact that misled two
  rounds four hundred sessions apart. One row off `inapplicable` to `partial`, one to the ledger's
  first ever **`silent`**, and `doc/ledger-and-claims.md`'s claim that `silent` is structurally zero
  corrected: it was zero because nothing was re-reading the rows that say nothing is owed.
- **812 — a flush is not a truncation.** A producer that calls `Z_SYNC_FLUSH` and never calls
  `deflateEnd` has written every byte of its data and no RFC 1951 final block, so `Damage::Truncated`
  — whose own words are true of it — reported a shortfall that did not exist, and four consumers
  branched on it. The fix replays the input through a throwaway raw decoder fed a final empty stored
  block, which puts no work at all on the healthy path. Corpus incomplete 66 → 63, damaged streams
  48 → 41, and the mechanism gone from the composition. Two things fell out of asking the question
  again: the flush marker is **four** bytes and not the five the todo wrote down, and
  `damaged_content_streams.rs` was wrong in *both* directions — it denied a witness that had been in
  the corpus the whole time.

**What 810, 811 and 812 share is the failure `CLAUDE.md` principle 1 is really about.** A lint that
stops at a workspace boundary, a status whose population is zero because nobody re-reads it, and a
report that fires on a condition the clause does not state are three instruments each reporting
success about something it never examined — and none of them could be seen from its own output.
809 is the same shape one level up: an exclusion whose reason ("a sandboxed script engine is a
separate project") had expired when `pdf-sandbox` and `viewer-confined` were built, and which
nobody re-read because nothing prints an expired reason.

## Owed, named

- **CI's verdict waits on the owner's push.** `main` is 95 commits ahead of `origin/main` and was
  not pushed; `origin/main`'s last run is a pre-existing failure. Nothing in this batch was judged
  by CI, and CI is the only machine where `RUSTFLAGS="-D warnings"` gates a push.
- **`doc/rfc/` awaits the owner's review**, untouched again this round. 809 names 0006 as the free
  number for the script-engine placement design without taking it.
- **`doc/todo/56` awaits the owner's decision, and nothing else.** The exclusion in `CLAUDE.md`'s
  closed list is the owner's to amend, and the todo states the amendment in the form they would
  have to ratify, counter-evidence included. **Its step 4 needs the owner to acquire ISO 21757-1:2020** —
  the round could establish that the standard exists and is normatively referenced, and could not
  read it.
- **`QUORRA_FEEDBACK` §40** is still open upstream.
- **`fuzz/seed_confined_wire.py` stops four discriminants short** of the wire's current vocabulary.
- **No target was fuzzed for its documented run length in 810.** The measurement that round made is
  a coverage comparison at 30 000 executions and `-runs=0`, not a fuzzing session, and the corpus
  levels it establishes are now the baseline a later round should beat rather than a clean bill.
- **The owner's `git stash drop`.** `stash@{0}` is the known-dead entry `doc/environment.md`
  documents; only the owner can drop it, and `tools/round.sh` will keep reporting a non-empty stack
  until they do.
- **The shared-scratchpad hazard 812 reported** stands as a hazard rather than a fix: the harness
  gives every parallel round the same scratchpad path, and this round named every file after itself
  (`gates-813`, `sweeps-813`) for that reason.
