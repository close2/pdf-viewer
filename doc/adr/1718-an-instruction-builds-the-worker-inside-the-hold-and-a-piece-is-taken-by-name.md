# 1718 — An instruction builds the worker inside the hold, and a piece is taken by name

Session 1443. Status: **accepted** and **built**. Finishes what ADR 1710 section 5 left: the traps
and the doc comments that still told a round to build the sandbox worker before the lock, and the
merge's two build lines that section 1 of it found redundant. Names, where the lock's cost is read,
the runs the rule line says are not walks (ADR 1706 section 4). Corrects the `foreign_corpus` gate's
written defect, whose diagnosis did not hold. Supersedes nothing.
Context: ADRs 1684, 1698, 1706, 1710; traps 10, 13, 109, 130.
Code: `tools/batch.sh` (`gates`), `tools/state.sh` (`lock_cost`), `crates/pdf-transform/tests/
foreign_corpus.rs` (`first_output`, `first_written`), the running sections of `crates/viewer-ui/tests/
launch_path.rs` and `crates/pdf-vfs/tests/read_corpus.rs`, traps 10 and 109, `doc/environment.md`'s
rule line. Tests: `tools/conformance/tests/bounded.rs` (the last three), `foreign_corpus.rs` (two),
`worker_features.rs` (two, section 5, a hand-over from round 1441).

## 1. An instruction is the walk's `--build`

**The premise, checked.** The brief named `pdf-model/tests/read_corpus.rs`; the file is
`crates/pdf-vfs/tests/read_corpus.rs`, and what it told a person to build first was its own package's
`pdf-vfs --bins`, which no walk needs, behind a `tools/bounded.sh` with no `--lock`. Both running
sections are now one locked command whose `--build` makes the other package's worker.

**Held.** `every_instruction_a_person_reads_builds_the_worker_inside_the_walks_hold` reads the tracked
text files the bare-`flock` sweep reads (2 007) for a line *opened* by `cargo build` — after a
comment's marker, behind assignments, or after `;` or `&&` — that names `pdf-sandbox` with `--bins`,
or `pdf-sandbox-worker`. Prose that names the build as what makes the worker opens no line with it
and is not an instruction to run it before anything. Eight lines in six files matched; the
`launch_path` one is re-spelled, and five files are held as a ratchet, each another round's:
`raster_golden.rs`, `doc/checks/fixed-documents.toml`, `doc/checks/launch-path.toml`, `doc/todo/02`
(three lines, one of them a measurement's `--bin pdf-sandbox-worker`) and `doc/todo/03`. Calibrated
by planting five shapes it names and five it passes, the wrapper's `--build`, the merge's gate line and
a prose mention among them. Trap 10's row and body and trap 109's say `--build`; a dev nextest's build
stays a plain one, since a crate-scoped test takes no lock.

## 2. The merge builds what its gates spawn, and nothing else

`build-vfs` and `build-confined` are gone. Cargo builds a package's binary targets when an integration
test of that package is selected (the Cargo book's *cargo test* target selection), so `t3-vfs_*` and
`t3-awkward` build their own workers. `every_program_a_merge_gate_spawns_is_built_ahead_of_it_and_no_
build_is_idle` holds both directions: a gate whose test *calls* `require_the_sandbox(` or asks
`Reference::Hayro` has a build of that package under its own profile ahead of it, and every build is
used by a gate after it. A call and not the name, because `turn_path.rs`'s exemption names the
requirement in its sentence, and the first reading named `t2-turn_path` for a release worker it does
not spawn. On the old script it named exactly the two removed lines; 17 gates spawn one.

## 3. A run that is not a walk is named

`tools/state.sh gates-cost` prints each run of the last batch that took the lock for a dev-profile
`cargo test` or `cargo nextest run` asking no `--ignored` test and not `--workspace` — tier 1's shape;
every walk the gates and the sections run states `--profile`, `--release`, `--ignored` or `--workspace`
— and sums their queue. Over batch sixty-seven's 128 lines: **10 runs, 2 838.9 s of queue**, all
round 1428's, the 1 820.8 s conformance run among them (which *waited* 1 820.8 s and held 8.9 s; the
brief's "ran" was the wait). Named and not refused: the lock is not that section's to give, and a run
that wants quiet for a reason of its own says `--clock`.

## 4. The `foreign_corpus` fault was a piece taken by its thread

**The written diagnosis did not hold.** It said `mutool show`'s answer under load reads as a short
array and proposed guarding the length mutool states against the members it prints. mutool 1.28.4
states no length: `…/Nums/2/length` prints `null`. And the 79 is not a truncation: page 2 of
`bug1997343.pdf` has `/StructParents 1`, and `ParentTree/Nums/4` has exactly 79 members, against page
1's 90. `split` writes its pieces across rayon, `MemorySinks` keeps them in the order they were
*opened*, and `first_output` took the first opened — so under load the bookmarks lane compared the
second chapter's page with the source's page 1. It now takes the output the report names first,
which is the first piece whatever the threads did; a unit test plants the outputs opened second
chapter first (the old reading answers the second), and one splits the document eight times. The
module's paragraph says so, and a §14.7 fault from this gate is a verdict again.

## 5. The worker's decoders keep their own features in the build that ships them

Taken from slot 4 (round 1441), with ADR 1714 section 2 as the reason: Cargo unifies a package's
features over the packages a build selects, `tools/batch.sh install` selects every member, and a
crates.io `hayro-jbig2` would have carried the reference renderer's `simd` into the worker.
`tools/conformance/tests/worker_features.rs` walks every package reachable from
`crates/pdf-sandbox/Cargo.toml`'s unconditional `[dependencies]` — the codecs, macros not entered —
and holds the features `cargo tree --workspace -e normal` gives each to those `cargo tree -p
pdf-sandbox` gives it. **Stable `cargo tree`, not the nightly unit graph**, so that it runs wherever
`cargo test -p conformance` runs, CI included; the two were read against each other: both give the
three `hayro` units `std`, `std` and none, and both give `hayro-jbig2` `fearless_simd, simd, std`
with `--features hayro-jbig2/simd`. That plant is the test's calibration, through Cargo itself, and
it is named as itself; a reader plant holds the walk.
