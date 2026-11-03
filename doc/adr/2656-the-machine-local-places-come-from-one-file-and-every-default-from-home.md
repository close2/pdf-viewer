# 2656 — The machine-local places come from one file, and every default from `$HOME`

Session 1911, a single round (instruments), the first on the server VM. Status: **accepted** and
**built** in `tools/places.sh`, the six scripts that source it, `tools/main-checkout.py`, and the
tests that need a place. Context: the owner, 2026-11-03, of the notebook agent's home directory
written into the tools: "bad practice and should be fixed anyways." ADR 2657 is this round's other
decision.

## The decision

Every machine-local place the tools use is defined once, in `tools/places.sh`, which a script
sources (`. "$(dirname "${BASH_SOURCE[0]}")/places.sh"`) and a Rust test reads through `bash -c '.
tools/places.sh; printf %s "$NAME"'`. Each name is the environment's where the caller set it, and
otherwise a default derived from `$HOME`. Two roots are the whole of what a machine decides:
`PDF_VIEWER_SCRATCH`, a directory `scratch` in the home directory, and `PDF_VIEWER_AGENT`, a
directory `agent` inside it. `PDF_VIEWER_BUILDS` is `cargo-target` inside the first, and every
other name is a file or directory inside one of the three: `HEAVY_WALK_LOCK`, `HEAVY_WALK_LOG`,
`HEAVY_WALK_REPLAY`, `BATCH_ARMS_ROOT`, `BATCH_GATES_LOG`, `ROUND_BRIEFS`, `DOTNET_CHECK_SAVED` and
`T88_CONFORMANCE_DATA` in the agent root; `BATCH_TARGET_DIR` and `BATCH_EXPORT_TARGET_ROOT` in the
builds root; `BATCH_WORKTREE` in the scratch root. The file's header and comments are the table,
and `bash -c '. tools/places.sh; printf "%s\n" "$NAME"'` prints any one; this ADR does not copy
them (ADR 2657 holds a record to naming no such path).

The names the tools already read (`HEAVY_WALK_*`, `BATCH_*`, `ROUND_BRIEF`, `T88_CONFORMANCE_DATA`)
keep their meaning; only their defaults moved. `tools/main-checkout.py`'s agent account defaults to
the account running it rather than a user name, and the `function_paint` spike reads this tree's
`doc/corpora-own/` through its manifest directory.

## Why one file, and why these defaults

- **One definition** because the same place was written in up to four scripts and two tests: the
  heavy-walk log in `bounded.sh`, `batch.sh` and `state.sh`, each with its own literal. A machine
  change then means finding every copy, which is how the notebook's directory survived the move to
  the VM. A variable each script read with its own `${X:-literal}` fallback would have kept the
  copies; the file is the only place a default is written.
- **A shell file** because six of the seven readers are shell scripts and the one Rust reader is a
  test that may run `bash`; a TOML file would need a parser in every script.
- **`:=`, not exported**: a script that runs another runs one that sources the file too, so both
  answer alike, and a caller's override reaches both through the environment.
- **The scratch directory** because `CLAUDE.md`'s VM section makes it the disk for what is large
  or temporary: arms, build directories and worktrees are. The notebook had no such disk, and its
  agent's home *was* the root; `PDF_VIEWER_SCRATCH=$HOME` there reproduces the old layout apart from
  the `agent/` level.
- **`bounded.sh --lock` makes the lock's directory** on first use, so a fresh machine needs no
  setup step before its first walk.

## Cost

A script is no longer readable without the file beside it, and a test that copies a script into a
sandbox copies `places.sh` too (`tools/conformance/tests/batch.rs`, which also points both roots
into the sandbox so no test writes the machine's gate log). The orchestration directory moves once,
from the VM's `AI` directory in the home directory to the agent root's default.
