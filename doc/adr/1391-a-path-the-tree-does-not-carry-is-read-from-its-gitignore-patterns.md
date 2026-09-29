# 1391 — A path the tree does not carry is read from its `.gitignore` patterns, and `check` reads the population `commit` stages

Session 1277. Status: accepted and **built**.
Context: `tools/conformance/src/gitignore.rs`, `tools/conformance/src/pointers.rs` (`NOT_CARRIED`,
`Reach::Ignored`, `Pointer::pattern`, `reach_of`, `collect`), `tools/conformance/src/bin/pointers.rs`,
`tools/batch.sh` (`population`, `untracked_paths`, `check_batch`), `tools/conformance/tests/batch.rs`.
Builds on: ADR 0372 (`--bin pointers`), ADR 1379 section 5 (which priced this), ADR 1313 (`commit`'s
population). Trap 25's shape: a hand list beside the file that decides.

## 1. The pointer sweep's not-carried rung

`NOT_CARRIED` was thirteen paths written beside `.gitignore` and `doc/.gitignore`, and seven of them
were there only because a pattern ignores them. ADR 1379 section 5 named the cost of deriving them as
a gitignore matcher this crate, with one dependency, did not carry. It carries one now: the subset of
gitignore(5) the tree's own files use — comments, `!`, a trailing `/`, anchoring by an inner `/`,
`*`, `?` and a whole-segment `**`, and a path ignored when a directory above it is. A bracket class
is matched literally, because no `.gitignore` here writes one.

- **A pattern is asked before the tree is.** Whether a checkout holds an ignored file is the
  machine's: `doc/*.pdf` resolved as live in the main checkout and not carried in a fresh clone. A
  pointer a pattern ignores is now `Reach::Ignored` in every checkout, printed with the pattern
  (`.gitignore: /doc/md`) and counted by it. Only paths under this tree's own heads are asked, so a
  pattern without a slash (`*.profraw`) cannot claim another project's path.
- **The walk reads each `.gitignore` as it enters that directory** and does not descend into a
  directory a pattern ignores, as git does not. The main checkout's `corpus-cache` and `tmp`, about
  ninety thousand files between them, were walked before.
- **What stays by hand is what no pattern covers, each with its reason in the constant's comment**:
  the four submodules (tracked as commits, not ignored), the owner's `doc/adr_revisit` (ignoring it
  would hide it from the owner's own `git status`) and `scratchpad` (kept visible so `check` can say
  what a round left). Thirteen became six.

Measured over this worktree (`cargo run --release -p conformance --bin pointers`, before then after):
not carried 1159 → 602 by hand and 966 by a pattern; absent 251 → 250; live 11515 → 11363 and
unrooted 6982 → 6779, the difference being `doc/*.pdf`, `tmp/` and `corpus-cache/` pointers that
now name the pattern that decides them. The counts move with siblings' prose; the command is the
number.

## 2. `check` and `commit` read one listing

`check` found added files with `git status --porcelain` and `awk '{ print $2 }'`, and `commit` with
`--porcelain=v1 -z`. The first prints a path holding a space or a non-ASCII character quoted and
escaped, so a CJK file name under `scratchpad/` began with `"` and passed the scratchpad filter as
an unexpected extension. `check`'s three added-file findings now read `untracked_paths`, the
untracked records of `population` — the function `commit` stages from — so the two are one
population by construction. `batch.rs` holds it with a file whose name has a non-ASCII character and
a space under `scratchpad/`, and one outside it that must be named as itself; against the old
script the test fails.

## 3. Cost

A pattern a later `.gitignore` adds changes the sweep's rungs without an edit here, which is the
point and is also a way for a pointer to leave the absent list unread; the per-pattern count is
printed so that a new line with a large count is visible. A newline in a path is the one spelling
`untracked_paths` cannot carry, and no round writes one.
