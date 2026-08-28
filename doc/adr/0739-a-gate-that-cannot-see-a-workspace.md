# ADR 0739 — A gate that cannot see a workspace

Status: accepted, 2026-08-28. Session 807. Cites no clause: this is an instrument about the tree
rather than about ISO 32000-2, and the conformance ledger is untouched. It belongs with ADR 0450
(a lint run without `RUSTFLAGS` is a weaker gate than the one that gates a push), ADR 0557 (a gate
that measured a build the tree had not finished producing) and trap 16 — every one of them an
instrument that reported success without having done its job.

## The finding, in one sentence

`cargo fmt --all --check` has never read a line of `fuzz/fuzz_targets/`, it does not say so, and it
exits 0 — so two rustfmt diffs sat in the tree under a green formatting gate.

## Why `--all` is not all

`--all` and `--workspace` mean *every package in **this** workspace*. Cargo decides which workspace
a manifest belongs to by walking up to the nearest ancestor manifest with a `[workspace]` table, and
`fuzz/Cargo.toml` declares one of its own:

```toml
# Not a workspace member: cargo-fuzz builds this with its own profile and sanitiser
# settings, and including it would apply those to the whole workspace.
[workspace]
```

That comment is right and the decision stands — a sanitiser profile leaking into the tree would be
worse than the hole it opens. What follows from it is that **every workspace-scoped command in
`doc/todo/02` §2 stops at `crates/` and `tools/`**, and each such command owes a second invocation
naming `fuzz/Cargo.toml`.

§2 already knew this for one of them. The line

```sh
RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins
```

was added after fourteen rounds in which the fuzz targets did not compile against the tree they
fuzz — `confined_wire` matched on an `Answer` shape two rounds had reshaped, and the only instrument
that built `fuzz/` was a CI job failing for an unrelated reason. Principle 3 makes that worse than a
compile error: fuzzing is meant to be continuous from the first parser commit, and between those
rounds there was none.

**The formatting line one row above it never got the same treatment**, for the same reason nobody
looked: a gate that is silent looks exactly like a gate that is clean.

## What was actually there

Two diffs, both under the `edition = "2024"` style the root `rustfmt.toml` sets — rustfmt *does*
read that config from `fuzz/`, because config lookup walks ancestors; it is the *invocation* that
never arrives:

- `fuzz_targets/display_list.rs`, an `assert!` whose arguments sit inside `max_width` and outside
  rustfmt's narrower call width, so it wants one argument per line;
- `fuzz_targets/x509.rs`, two `use` lines in the order `reorder_imports` does not put them in.

Both entered in the days before this one, and every round since ran `cargo fmt --all --check` and
watched it pass.

## How far the hole goes, measured rather than assumed

The question "which files in this tree does the formatting gate not read?" was answered by
enumeration rather than by reasoning about workspaces: `cargo fmt --all -- --emit stdout` prints the
path of every file rustfmt formats, and the set difference against `git ls-files '*.rs'` is exactly
the fuzz targets — nothing else in the tree is invisible. That matters because the *other* way a
file escapes rustfmt is not a workspace at all but the module graph: rustfmt walks `mod`
declarations from each target's root, so a `.rs` file no `mod` reaches is equally unseen. This tree
has none, and the measurement is what says so.

Two by-products of doing it that way are worth keeping, because both nearly produced a wrong answer:

- **`grep -oP` silently dropped a match** in the middle of a 370,000-line stream, which put a file
  in the "unseen" column that the gate demonstrably reads. It was caught by planting a defect in
  that file and watching `cargo fmt --all --check` fail on it — trap 13 applied to the instrument
  rather than to the fix.
- **`sort`'s collation** put seven more files there. The answer above was recomputed in one process
  with one comparison, and the two artefacts vanished.

An enumeration is itself an instrument, and this one needed calibrating twice before it was
believed.

## The decision

Three parts.

**One: take the two diffs.** By running rustfmt where nothing had run it.

**Two: add the missing line**, beside the check line it belongs with rather than at the top of the
block, because the two exist for one reason and read better together:

```sh
cargo fmt --manifest-path fuzz/Cargo.toml --check
```

And correct §2's own claim while there. It said *the first four lines are the core and every round
runs them*, and five were owed for as long as the second `fuzz/` line has existed — the prose under
the map said so ("now every round runs it") while the sentence above it counted four. Now it counts
the two `fuzz/` lines as core and says why.

**Three: make the population derived**, in `tools/conformance/tests/workspaces.rs`. The fix above is
one line, and one line is exactly the kind of thing a tree grows a second hole in: the next crate
kept out of the workspace — a second fuzz crate, a benchmark harness with its own profile, a
`no_std` target — arrives with nobody remembering that `--all` is not all.

So the gate asks cargo. For every `Cargo.toml` the index tracks, `cargo locate-project --workspace`
answers which workspace root governs it; the distinct roots that come back are the tree's
workspaces, by the same rule cargo itself applies, and each must be named by a `cargo fmt` line of
§2 **and** by one of its `cargo clippy`, `cargo check` or `cargo build` lines. A `[workspace]`-table
grep would agree today and would be this project's own reading of cargo's rules rather than cargo's.

It costs about a second, and it lives in `tools/conformance/` for the reason `sandbox_gates.rs` and
`submodules.rs` do: that is the crate whose gates read the repository's own files rather than a PDF.

## What the gate found on its first run, which is the argument for having written it

It failed. `fuzz/Cargo.toml` was reported *uncompiled* — while the sequence compiles it — because
two of §2's lines begin `RUSTFLAGS="-D warnings"` rather than `cargo`, and one of those two is the
line that compiles `fuzz/`. A parser anchored on `cargo` at the start of the line cannot see it, and
the quoted value's own space defeats the obvious repair of skipping leading `KEY=value` words.

That is worth more than the bug it was. **The reason this hole existed is that a workspace-scoped
word looked like it covered everything, and the reason the gate's first reading was wrong is that a
command-shaped line looked like it started with the command.** Both are the same mistake about a
surface, and only running the thing tells them apart.

## Two things this deliberately does not do

- **It does not lint `fuzz/`.** The sequence checks that the targets compile; `cargo clippy
  --manifest-path fuzz/Cargo.toml` would be a different and larger decision, because that crate does
  not take `[lints] workspace = true` and so has never been under this tree's lint levels at all.
  Adding it is a round's subject, not a footnote to this one — and the gate above will not ask for
  it, because a `check` line answers the compiling half by design.
- **It does not generalise to "every tool must see every file".** The property asserted is about
  *workspaces*, which is the mechanism that actually produced the defect, and it is checked against
  cargo. A gate asserting something broader would be asserting something this round did not measure.
