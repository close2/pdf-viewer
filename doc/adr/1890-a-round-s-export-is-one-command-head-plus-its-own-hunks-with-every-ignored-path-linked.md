# 1890 — A round's export is one command: HEAD plus its own hunks, every ignored path linked, its own build directory

Session 1527, an instruments round of batch eighty-three. Status: **accepted** and **built**.
Context: ADR 1650 (`arms`, the shape copied), ADRs 1311, 1841, 1860; traps 50, 114, 129, 150, 153,
162. Code: `tools/batch.sh` (`export_round`, `export_sums`). Tests: `tools/conformance/tests/batch.rs`
(`export_is_head_plus_the_rounds_own_hunks_with_every_ignored_path_and_its_own_target`,
`export_refuses_a_path_with_no_change_and_a_patch_of_another_tree_and_keeps_the_last`,
`export_sums_hashes_every_executable_the_exports_build_directory_holds_and_nothing_else`).

## 1. The question

Six rounds edit one worktree, and a round's clock hold or gate has to measure its change alone. So
each one built an export by hand: `git archive HEAD`, its own files copied in, the corpora linked,
a `CARGO_TARGET_DIR`. Four rounds of batch eighty-two did this, and each step of it is a trap that
has already been sprung. The archive drops the corpora (trap 150) and every other gitignored path
(trap 162: `turn_path` "measured nothing" without `doc/PDF20_AN001-BPC.pdf`). A shared file copied
from the worktree brings a sibling's hunks with it (trap 153). An export under the worktree builds
into the batch's directory (trap 129). The archive's commit-time mtimes make a rebuild hand back
the previous binary (trap 114). Two exports sharing a directory give the same binary twice (trap 50).

## 2. Decision

`tools/batch.sh export <session> [--as <name>] [--patch <file>]... [<path>...]` builds
`scratchpad/r<session>/<name>/` (the default name is `export`), and
`tools/batch.sh export-sums <session> [--as <name>]` hashes what was built there.

1. **HEAD's archive, extracted at today's time** (`tar -m`), so every source is newer than the
   last build's artefacts.
2. **Every submodule, and every path `git status --ignored` names in the worktree, is linked in**,
   by `realpath`. The population is the worktree's own, not a list (trap 25). Three kinds are left
   out, each for its trap: `.cargo/` (129), a `target` directory, and `__pycache__` (114).
3. **A path argument takes the worktree's file whole**, and so does every changed path under a
   directory argument. A name that matches no changed path is refused before anything is written,
   because that export would be HEAD's and would read as the change (trap 25). A deletion stays a
   deletion. The copy takes today's time, not the file's own.
4. **A shared file goes in as `--patch`, the round's own hunks.** The patch is checked against HEAD
   in an index of its own before anything is written. A patch made against the worktree's copy has a
   sibling's lines in its context, and is refused. It is applied with git's repository search
   stopped at the export: below the worktree, `git apply` treats the export as a subdirectory and
   skips every path outside it without a word. A file is taken whole or patched, never both.
5. **Its own build directory**, `/home/AI/cargo-target/r<session>-<name>`, named in the export's own
   `.cargo/config.toml`, which cargo reads before the worktree's. `--as head` gives a second tree and
   a second directory, for an A/B.
6. **A record beside the tree**, `scratchpad/r<session>/<name>.export`, rather than in it, where a
   sweep would read it. It names the commit, the target directory, each path taken and each
   patch's `--numstat`. It counts the worktree's changed paths left at HEAD and the paths linked.
   `export-sums` replaces its `built:` lines with the SHA-256 of every executable the target holds:
   each profile's programs, examples and test binaries.

Nothing here walks or builds, so nothing here takes the lock. The round's build in the export is
its own walk.

## 3. Calibration

The first test plants a sibling's edit, a shared file both slots edited, a new directory, an ignored
specification, the batch's `.cargo/`, and a file the commit dates 2001. It fails with `tar -m`
taken out, and again with the ceiling taken out of `git apply`. On the batch's own worktree,
`export 1527 crates/viewer-ui/tests/launch_path.rs` took 0.6 s. It linked 43 ignored paths and 7
submodules and left 3 changed paths at HEAD. A blob-by-blob comparison of the 5 035 tracked files
against HEAD found one that differed: the named one.

## 4. What it costs

A re-export rebuilds every workspace crate in that build directory, because every source has a new
time. That is the price of item 1, and it is paid in that one directory. The first build of an
export is cold, in a directory of its own.
