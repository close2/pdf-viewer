# 1511 — The merge installs what a person runs, and writes the commit beside it

Session 1338. Status: accepted. Amends `doc/todo/02` section 5 (a round no longer installs) and
section 8 step 5 (the step between the fast-forward and `close`). Context: ADR 0222 (one
invocation), ADR 1440 (a batch builds apart from the main checkout), ADRs 1451 and 1463 (what the
batch directory holds), trap 15 (a binary from a neighbour's build directory). Code:
`tools/batch.sh` (`install_batch`, `install_binaries`, `install_libraries`), `tools/state.sh`
(`binaries`), `tools/round.sh` (check 3), `tools/conformance/tests/batch.rs`. Prose:
`doc/running-the-viewer.md`, `doc/todo/02` sections 5 and 8, `doc/environment.md` (the prune
commands).

## 1. What was there

`doc/running-the-viewer.md` said a round's last step copies three binaries from
`/home/AI/cargo-target/quorra/release/` into `target/`. That directory had not been built in since
2026-09-01 and is gone; `doc/todo/02` section 5 said the same thing every fifth round with ten names.
Neither could happen: rounds run in a batch worktree and may not write in the main checkout, and a
relative `target/` from a worktree is the worktree's own, which dies at `close`. Read on 2026-10-02,
the main checkout's `target/` held all ten programs and both libraries dated 2026-09-11 15:16 —
`main` was at `44c77ef5` then and is 79 commits past it — and `safedocs` from 2026-08-18. Nothing
said which commit any of them was; `quorra --version` opens a file called `--version`.

## 2. The decision

**The orchestrator installs, once a batch, with one command**: `tools/batch.sh install`, after the
fast-forward and before `close`. It refuses a worktree holding uncommitted work and a branch whose
HEAD is not `main`'s, so nothing is installed that no commit describes; builds the programs in one
`--release` invocation and the libraries in a second (`--bin` cannot name a library), in the
directory Cargo names in the worktree — asked, never written down; installs each into the main
checkout's `target/` with mode 775, because that checkout is shared through the `coders` group; and
writes `target/installed-from`: the commit, its subject, the date and directory, and each file's
SHA-256 in `sha256sum` form. It prints each path with the commit, and names what `target/` holds
that it did not install.

**The names are written once**, in `tools/batch.sh`. `tests/batch.rs` holds them against the
manifests: every binary target of a package under `crates/` is installed, every name installed is a
binary target, and every package building a `cdylib` is installed — so a program added under
`crates/` fails the test rather than going stale under `target/`. The ten and two are section 5's,
not the six the brief named: installing six would leave `quorrafs` and `pdf-vfs-worker`,
`quorra-retrieve`, `quorra-transform` and both libraries from September beside October's viewer.

**A round installs nothing, and before a measurement builds `--release` in its own directory and
runs that** — section 5's staleness rule kept, its install half moved to the merge.

**`install` writes, and is not a state section.** `tests/read_only.rs` says so in its header; it
writes exactly the main checkout's gitignored `target/`, which `tests/batch.rs` checks against a
throwaway repository by `git status --ignored` showing `target/` and nothing else. `tools/state.sh
binaries` now reads the main checkout's `target/` rather than this tree's, prints the record, how
many commits `main` is past it, and whether every listed file still has its checksum;
`tools/round.sh` fails while `main` is past it.

## 3. What it costs

One `--release` build of twelve whole-graph fat links per batch, in a directory whose `release`
profile already holds its dependencies (ADR 1463), on the orchestrator's clock. Not measured here:
the first run is the merge's, and its wall time belongs in that batch's commit body beside the
gates line.
