# 1732 — The five held instructions build the worker inside the hold

Session 1448. Status: **accepted** and **built**. Finishes ADR 1718 section 1: the five files its
sweep held as a ratchet, each another round's, now spell the walk's worker as the wrapper's own
`--build`, and the held list is empty. Supersedes nothing.
Context: ADRs 1620, 1684, 1710, 1718; traps 10, 13, 25, 109.
Code: `crates/pdf-model/tests/raster_golden.rs` (its running section), `doc/checks/fixed-documents.toml`
and `doc/checks/launch-path.toml` (their headers), `doc/todo/02` section 2 (tiers 2 and 3, the
`launch_path`, `awkward_classes` and `pdfref-hayro` notes) and section 5's measurement block,
`doc/todo/03` section 20's merge command. Test: `tools/conformance/tests/bounded.rs`
(`HELD_WORKER_BUILD_INSTRUCTIONS`, now `[&str; 0]`).

## 1. An instruction for a walk is its section, spelled out

**The decision.** A document that tells a person to run a walk writes the one command
`tools/state.sh --round <session> <section>` runs — `ulimit -u 8192;`, `tools/bounded.sh --lock`,
the lane its section declares (`--tree 6`, `--tree 12` or `--clock`), the worker as `--build '<the
command's profile> -p pdf-sandbox --bins'` and any second program as a second `--build`, then `--`
and the gate — and names the section beside it. Not a build line above the gate: that build is as old
as the moment the walk stopped queueing, and five siblings edit the tree meanwhile (trap 109). Not
the section alone either: a person regenerating `raster_golden.tsv` sets an environment variable on
the gate, and the spelled-out command is where it goes.

**`doc/todo/02`'s tier blocks stay lists of gates.** Their first line is the wrapper's shape with
`<the line>` after its `--`, and each line keeps its `cargo test …` words, because four conformance
tests read those lines (`sandbox_gates`, `state_sections`, `batch`, `workspaces`). One of them,
`sandbox_gates`, reads `-p` and `--test` from anywhere on a `cargo test` line, so a trailing comment
there names no package: the first spelling of `launch_path`'s comment carried `-p pdf-script-worker`
and the test failed by name on it.

## 2. Tier 3's two own-package builds go

`cargo build --profile gates -p pdf-vfs --bins` and `-p viewer-confined --bins` stood in tier 3 as
"trap 10 again". ADR 1718 section 2 took them out of the merge because Cargo builds a package's
binaries when one of its integration tests is selected; the lines here said the opposite of the merge
and are gone, with the `awkward_classes` note saying why. The `pdfref-hayro` build is the oracle
walk's second `--build`, as `section_oracle` spells it, and the note's argument for placing its line
after the corpus gate — compile overlap of two separate commands — no longer has two commands to
place.

## 3. Calibrated

The sweep reads 2 011 tracked text files and holds 0, owes 0. Planted back by reversing this round's
own patch on `doc/checks/launch-path.toml`, it names `doc/checks/launch-path.toml:[8]` and fails;
re-applied, it passes (trap 13).
