# 807 — A gate that cannot see a workspace

Date: 2026-08-28. Branch `round-807`, from `main` at `5efead7b`. Parallel round, worktree `r807`.
ADR: [0739](../adr/0739-a-gate-that-cannot-see-a-workspace.md).
Touched: `fuzz/fuzz_targets/display_list.rs`, `fuzz/fuzz_targets/x509.rs`,
`doc/todo/02-every-round.md` (§2), `doc/traps/instruments-and-reports.md` (trap 23, new),
`doc/HANDOVER.md` (its index), and two new files — `tools/conformance/tests/workspaces.rs` and
`doc/adr/0739` — plus this one.

## The subject, and why it was taken over what the instruments ranked

The batch's general-improvement round, told to let the instruments name the subject and offered two
standing items about instruments that report success without doing their job. The instruments were
asked first:

- **The sweeps.** All sixteen argument-free conformance sweeps and `spec-errata`'s `check`, `moved`
  and `applied` were run over the pristine tree before anything was written. Every hit is a shape
  the sweeps' own catalogues describe as noise — the `parts` sweep's `doc/todo/02` hit is that
  sweep's own sentence about the population it exists to catch, quoted as history.
- **The ledger.** 875 rows, 0 new, and this round adds no verb to it.
- **The corpus and oracle lanes** are where 804, 805 and 806 are working, and the brief rules them
  out for this round.

So the standing pair was taken, and of the two the formatting one was chosen because it turned out
not to be small: **the question is not "why are these two files unformatted" but "what else can the
formatting gate not see?"**, and nothing in the tree had ever asked it.

## The finding, in one sentence

`cargo fmt --all --check` has never read a line of `fuzz/fuzz_targets/`, because `fuzz/Cargo.toml`
declares a `[workspace]` table of its own and `--all` means *every package in **this** workspace* —
and it exits 0 while not doing it.

## What the measurement said

`cargo fmt --all -- --emit stdout` prints the path of every file rustfmt formats. Against
`git ls-files '*.rs'` the difference is **exactly the fuzz targets** — the module-graph hole (a
`.rs` file no `mod` declaration reaches) is empty in this tree, which is a fact nobody had.

Getting that answer took three attempts and two of them were wrong in the direction that would have
sent this round chasing a phantom. `grep -oP` dropped a path line in the middle of a 370,000-line
stream and put `crates/viewer-qt/tests/unsafe_position.rs` in the unseen column; planting a
formatting defect in that file and watching `cargo fmt --all --check` fail on it is what said
otherwise. Then `sort`'s collation added seven `crates/viewer-confined/examples/` files to the same
column. Recomputed in one process with one comparison, both artefacts vanished. **An enumeration is
an instrument, and this one needed trap 13 run against it before its output was believed.**

## What changed

1. **The two diffs**, taken by running rustfmt where nothing had run it. Both entered in the days
   before this round and every round since watched `cargo fmt --all --check` pass over them.
2. **The missing line** in §2, beside the `cargo check --manifest-path fuzz/Cargo.toml` line it
   belongs with — the two exist for one reason. §2's own "the first four lines are the core" is
   corrected in the same edit: five were owed for as long as the second `fuzz/` line has existed,
   and the prose under the map already said so while the sentence above it counted four.
3. **`tools/conformance/tests/workspaces.rs`**, so the next workspace kept out of the tree's
   own fails on the day it is added. Its population is derived rather than listed:
   `cargo locate-project --workspace` is asked, for every tracked `Cargo.toml`, which workspace root
   governs it, and every root that comes back must be named by a `cargo fmt` line of §2 *and* by one
   of its `clippy`, `check` or `build` lines.

4. **Trap 23**, in the group whose rounds spring it, because the lesson outlives the one line that
   fixes it: a workspace-scoped flag is a claim about the manifest graph, not about the directory,
   and the module graph is a second way a file escapes a tool that no gate here closes. The
   handover's index gained its row — **and trap 16's, which the group's own row had never listed**
   though the file has carried it since it was written.

**The gate failed on its first run, correctly**: it reported `fuzz/Cargo.toml` uncompiled while the
sequence compiles it, because two of §2's lines begin `RUSTFLAGS="-D warnings"` rather than `cargo`
— and the quoted value's own space defeats the obvious repair of skipping leading `KEY=value` words.
The same mistake about a surface as the defect it was written for.

## Trap 13, four plants, above commit `82d60db4`

Each was planted, run, and reverted; the messages are distinct, so no arm swallows another's case
(796's lesson).

| plant | gate says |
|---|---|
| the new `cargo fmt --manifest-path fuzz/…` line deleted from §2 | `no cargo fmt line … reads them: ["fuzz/Cargo.toml"]` |
| the `cargo check --manifest-path fuzz/…` line deleted from §2 | `no cargo clippy, cargo check or cargo build line … : ["fuzz/Cargo.toml"]` |
| a third workspace added (`bench/Cargo.toml` with `[workspace]`, tracked) | the formatting arm, naming `bench/Cargo.toml` |
| the same, with a `fmt` line for it added and no compiling line | the compiling arm, naming `bench/Cargo.toml` |

And the defect the round exists for, planted in `fuzz/fuzz_targets/lexer.rs`:
`cargo fmt --all --check` exits **0**, the new line exits **1** and prints the diff.

## Gates, as the change → gate map assigns

Documents plus `tools/conformance` plus `fuzz/`: the core four, the two `fuzz/` lines, and
`cargo test -p conformance`. Not a pixel-changing round and not a fifth round; the corpus, oracle,
quorra, census and fixed-document lines are not in this change's reach and were not run.

| line | result |
|---|---|
| `cargo fmt --all --check` | silent |
| `cargo fmt --manifest-path fuzz/Cargo.toml --check` | silent (it was not, before this round) |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | silent, beside the documented cold-build `viewer-qt@0.1.0:` gcc lines |
| `cargo nextest run --workspace` | all run, all passed, 18 skipped |
| `cargo test --workspace --doc` | passed |
| `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins` | silent |
| `cargo test -p conformance` | passed, three test binaries and the citation suite |

## The §4 sweeps, before and after, every delta accounted

Nineteen sweeps run over the pristine `main` checkout and again over this worktree. Four files
differ and none is a finding:

- **`applied`** reads 14 more places — the prose this round added. `0` name an erratum on both
  sides, and the `#NNN` token count is identical.
- **`pointers`** counts 3 more path pointers, all live, all this round's new file. Its other
  difference is environmental: the primary checkout has a gitignored `tmp/hayro/` that a worktree
  does not, so three ADR pointers read `looked in: tmp/hayro/…` there and `no file of that name`
  here. Not caused by anything in this branch.
- **`ledger`** differs only in the absolute path it prints. 875 rows, 0 new, both sides.
- **`parts`** reports its one `doc/todo/02` hit at line 359 instead of 337, because this round added
  lines above it. Same hit, same sentence, and it is the sweep quoting its own reason for existing.

`retired` exits 1 on both sides, unchanged.

## What was left alone, deliberately

- **`fuzz/` is still unlinted.** The sequence checks that the targets compile; putting them under
  `clippy` is a larger decision, because that crate takes no `[lints] workspace = true` and has
  never been under this tree's lint levels. ADR 0739 says why that is a round's subject rather than
  a footnote to this one.
- **The second standing item** — a fuzz target whose exit status says nothing about whether it
  fuzzed (round 800's `page`, 86,912 iterations at zero coverage, exit 0, for want of seeding) — was
  not taken. It is the same failure shape and it deserves the same treatment: a round, not half of
  one.
