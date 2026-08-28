# ADR 0742 — A fuzz run that exits zero without fuzzing

Status: accepted, 2026-08-28. Session 810. Cites no clause: like ADR 0739 this is an instrument
about the tree rather than about ISO 32000-2, and the conformance ledger is untouched. It belongs
beside 0739 (a gate that cannot see a workspace), 0450 (a lint run without `RUSTFLAGS` is a weaker
gate than the one that gates a push), 0557 and trap 16 — every one of them an instrument that
reported success without having done its job.

ADR 0739 left two holes in writing and named them a round's subject rather than a footnote. This is
that round, and it takes both, because they are one shape seen twice: **`fuzz/` was outside the
tree's lint levels, and a fuzz run's exit status says nothing about whether it fuzzed.**

## Part one — `clippy` had never judged a fuzz target

### What was there

`fuzz/Cargo.toml` took no `[lints] workspace = true`, so `pedantic`, `arithmetic_side_effects`,
`unwrap_used`, `missing_docs` and the rest stopped at the workspace boundary exactly as `--all` did.
Putting the crate under them surfaced **thirty-three findings across nine of the fifteen targets**,
and they fall into three kinds with three different answers.

**Five are arithmetic in a target's own counters, and they are the ones that mattered.**
`fuzz/Cargo.toml` sets `overflow-checks = true` in its release profile — deliberately, with a
comment saying why: "an arithmetic bug that wraps silently is exactly what fuzzing should surface".
The consequence nobody had drawn is that **the same setting applies to the target's own
arithmetic**, so `tokens += 1` in `lexer.rs` is an abort waiting to be reported as a crash in the
lexer. Each is now saturating, and each carries the sentence saying why the saturation cannot hide
what the assertion beside it exists to catch:

```rust
// Saturating rather than `+=`: `fuzz/Cargo.toml` keeps overflow checks on in the release
// profile, so a counter that wrapped would abort *inside the target* and libFuzzer would
// file it as a crash in the lexer. Saturation hides nothing, because the assert below
// still fails at the ceiling.
tokens = tokens.saturating_add(1);
```

**Twenty are `panic!` and `expect`, and they are the target's vocabulary rather than a defect.** A
fuzz target states a property by failing; `expect("the same bytes parsed once already")` *is* the
idempotence check. So each such target takes a crate-level `#![expect(…, reason = …)]`, which is the
form `crates/pdf-model/tests/actions.rs` already uses for test code and which trap 7 requires over
`#![allow]`. The `clippy.toml` at the tree's root grants `allow-panic-in-tests` and its two
siblings, and a `#![no_main]` fuzz binary is not `#[cfg(test)]`, so the exemption does not reach it
— which is correct: an exemption stated at the file makes the exception visible, and the blanket one
would not.

**The rest are pedantic style**: three `doc_markdown` misses that wanted the tree's own backticked
`` `XObject` ``, three `match_same_arms` where `Ok(false)` and a named refusal are two answers with
one consequence and a comment on each saying which is which, and one `items_after_statements`.

### What keeps it closed

Two properties, because a lint level travels by a different road from a command, and closing one
without the other closes nothing.

**The command.** §2's line for `fuzz/` was `cargo check --manifest-path fuzz/Cargo.toml --bins`; it
is now `RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets`. A
`check` cannot see a lint, and the `RUSTFLAGS` is ADR 0450's rule, not decoration. The line count of
§2's core does not change: `clippy` answers the compiling question `check` answered.

**The levels.** `[lints] workspace = true` resolves against *this* workspace and cargo offers no way
to point one workspace's packages at another's table, so a second workspace's lint levels are
unavoidably a **copy** — and a copy is exactly what this project's own rule about two documents
stating one thing predicts the fate of. `tools/conformance/tests/workspaces.rs` therefore compares
the two `[workspace.lints.*]` tables entry by entry and fails on any level one states and the other
does not, in either direction.

### The gate 807 wrote, and whether it should now demand a lint line

807 wrote `workspaces.rs` so that every workspace root cargo names must be covered by a `cargo fmt`
line of §2 and by one of its `cargo clippy`, `cargo check` or `cargo build` lines, and it
deliberately did **not** demand a lint line — a `check` answered the compiling half by design.

It should now, and the argument is 807's own applied one step further. That gate's population is
*workspaces*, because a workspace is the mechanism that produced the defect; and the mechanism has
now produced it twice, once for formatting and once for linting. `cargo clippy --workspace` stops at
the boundary for precisely the reason `cargo fmt --all` does. A gate that guards one and not the
other is guarding a symptom.

So the third arm demands a `cargo clippy` line naming each root, under `RUSTFLAGS="-D warnings"` —
and the fourth compares the lint tables, because a run under levels the tree does not share is an
instrument reporting success against a different standard, which is the same failure wearing the
other half of the costume. Reading the environment prefix is what the third arm needs and is a
by-product 807 already had to build: two of §2's lines begin `RUSTFLAGS="-D warnings"` rather than
`cargo`, and that is why `cargo_invocations` keeps the prefix instead of only skipping it.

The generalisation is still refused, on 807's own terms. The property asserted is about
*workspaces*, and it is checked against cargo. A gate asserting "every tool must see every file"
would assert something this round did not measure.

## Part two — an exit status is not a coverage figure

### The measurement, which is the whole of the argument

Every one of the fifteen targets was run twice: 30 000 executions from an **empty** corpus
directory, and the tree's own corpus **executed once** (`-runs=0`) so that the seeds' own coverage
is the figure rather than a fuzzing session's. libFuzzer's own final line is the instrument.

| target | from nothing | its corpus, executed once |
|---|---|---|
| `lexer` | cov 256, ft 853 | cov 333, ft 2026 |
| `object` | cov 471, ft 1582 | cov 711, ft 4232 |
| `document` | **cov 103, ft 182** | cov 5421, ft 24 133 |
| `page` | **cov 103, ft 182** | cov 41 422, ft 218 862 |
| `cmap` | cov 457, ft 1077 | cov 560, ft 2427 |
| `crypt` | cov 1008, ft 1795 | cov 1252, ft 3966 |
| `variable_text` | cov 2647, ft 3581 | cov 7138, ft 21 787 |
| `forms_data` | cov 264, ft 640 | cov 488, ft 1375 |
| `sfnt` | cov 137, ft 275 | cov 500, ft 1641 |
| `xmp` | cov 928, ft 1615 | cov 2244, ft 8922 |
| `fragment` | cov 318, ft 773 | cov 523, ft 2560 |
| `cms` | cov 107, ft 333 | cov 487, ft 1211 |
| `confined_wire` | cov 2170, ft 2568 | cov 6425, ft 15 178 |
| `display_list` | cov 631, ft 906 | **no corpus directory existed** |
| `x509` | cov 114, ft 214 | cov 883, ft 1446 |

Two things to read off it. **Every target's corpus is worth between two and a thousand times what
the fuzzer invents**, so none of these is a target that would be fine unseeded. And **the row that
settles it is `page`, settled by the row above it**: from nothing `page` reaches cov 103 and ft 182,
*the same two figures `document` reaches, to the unit*. The two targets share a prefix — both hand
the fuzzer's bytes to `Document::open` — and diverge only once a document parses well enough to have
a page tree, so identical figures mean **`page` never once entered `pdf_model::interpret`**. That is
the entire reason the target exists: ADR 0264 built it because `nm` said twelve of the thirteen
binaries did not contain the interpreter, and unseeded the thirteenth does not either in any sense
that matters. Its corpus takes it to ft 218 862, which is twelve hundred times as much.

`display_list` had no corpus directory at all when this was measured — its seeder is
`viewer-confined`'s `list_over_the_wire` example rather than a Python script, and it had never been
run here. Seeded from the pdf.js documents it reaches cov 1796, ft 7294 against 631 and 906 from
nothing.

So round 800's `page` at 86 912 iterations, `cov: 0 ft: 0 corp: 0`, exit 0 was not an anomaly to be
explained away. It is what this target does with no seeds, and no amount of wall clock changes it: a
fuzzer will not invent a header, a cross-reference section, a page tree and a resource dictionary
that agree with each other.

### Why the corpus was empty, which nobody had asked

`fuzz/corpus` and `fuzz/artifacts` are in `.gitignore`. `tools/worktree.sh` links the gitignored
data a worktree needs — `doc/md`, the two submodules, `corpus-cache` — and **it did not link
these two**. So a parallel round's worktree had *no fuzz corpus at all*, and every fuzz run made
from one since worktrees began started from nothing.

This was measured rather than reasoned about: the sibling round's worktree has no `fuzz/corpus`
directory, and this round's first measurement pass reported `files=0` for all fifteen targets while
the primary checkout holds thousands apiece. **A round in a worktree fuzzed the recovery scanner and
reported it as having fuzzed the interpreter, and exited 0.** That is the same sentence as ADR
0739's, with a different instrument in it.

Two consequences follow and both are now written where a round meets them. A worktree links the two
directories, so it fuzzes the tree's corpus and a crasher it finds lands where the next round will
see it — two rounds appending to one corpus is what `-jobs` does inside one, so sharing is the
behaviour principle 3 asks for and a per-worktree copy would be the wrong answer. And **whether a
target is seeded is a fact about this disk, not about the repository**: no gate can read it out of
the tree, a clone has to re-seed from the scripts in `fuzz/`, and `display_list` — whose seeder is
`viewer-confined`'s `list_over_the_wire` example rather than a Python script — has never been seeded
here at all.

### The decision: `tools/fuzz.sh`

A wrapper, because the two questions a bare `cargo fuzz run` does not answer are cheap to ask and
neither is a judgement call.

```
tools/fuzz.sh <target> [--from-nothing] [-- <extra libFuzzer arguments>]
tools/fuzz.sh --list
```

**Before**: a target whose corpus directory is empty stops the run. This is derived rather than
declared — no list of "targets that need seeds" is kept anywhere, because in this tree *every*
target has a corpus and an empty one means it was lost rather than chosen. `--from-nothing` is how
the deliberate case says so, out loud, in the command line a round writes down.

**After**: libFuzzer's own last `cov: … ft: … corp: …` is read back, and a run whose `ft` is zero
fails whatever its exit status was — as does a run that printed no such line at all, which is what a
build failure, a refused flag and a crash on the first input all look like from outside.

**And what that second condition is worth had to be measured, because trap 11 is about exactly
this.** Asked of an ordinary run from an empty corpus, `ft` is *never* zero: libFuzzer executes the
empty input at `INITED` and that alone reaches the harness — `display_list` from nothing reports
`cov: 31 ft: 32`, and the wrapper correctly passes it. So the arm would be decoration, were it not
for where round 800's figures actually came from. Reproduced: `page` under `-fork=6` against an
empty corpus prints

```
#59025: cov: 0 ft: 0 corp: 0 exec/s: 29512 oom/timeout/crash: 0/0/0 time: 2s job: 1 dft_time: 0
INFO: exiting: 0 time: 2s
```

— zero on all three counters, tens of thousands of iterations, exit 0, which is round 800's
sentence word for word. **A fork-mode parent's counters are the shared corpus's**, and a shared
corpus that starts empty and is fed by children who find nothing worth keeping stays at zero. Since
`-fork` is what makes `page` runnable at all (§2's own note: ten to thirty executions a second under
the sanitiser), the arm fires exactly where the tree's slowest and most valuable target is, and
nowhere else. Calibrated by running it: `cargo-fuzz` exits 0 and the wrapper exits 1, naming the
target.

**And the invocation comes out of `doc/verify.md`.** That file states one
`cargo +nightly fuzz run <target>` line per target with the flags that target needs — `page` is
forked over six processes, `x509` runs a million times — and the wrapper runs *that line*. Two
places stating one command is how they drift (ADR 0232 §4), so the document owns it and the tool
reads it; a target with no line there is refused, which is what makes a new target arrive
documented. It is the same argument `workspaces.rs` makes about §2 and `sandbox_gates.rs` about the
gate sequence.

`--list` is the report half, and it is where the two facts a round actually needs live: every
target, how many seeds it has *here*, and the invocation the document gives it. It is how this
round found that `display_list` has none, and it costs no build at all.

### What this does not do, deliberately

- **It is not a §2 gate.** Fuzzing is `doc/verify.md`'s, it needs nightly and a sanitiser build, and
  the slowest target is an hour. What §2 gains is the lint line above.
- **It does not ratchet coverage.** A floor per target would be a ratchet on a *gitignored*
  directory — a number that falls because a machine was reinstalled, not because the tree moved. The
  honest check is the one that needs no history: a run that found no features found none.
- **It does not seed anything.** The seeders exist and are documented per target; a wrapper that
  quietly re-seeded would be a wrapper deciding what corpus a round fuzzes, which is the round's
  decision and an expensive one.
