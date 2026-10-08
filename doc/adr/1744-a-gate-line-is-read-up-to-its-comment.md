# 1744 — A gate line is read up to its comment

Session 1454. Status: **accepted** and **built**. Finishes the habit round 1448 wrote down after
ADR 1732 section 1: the sweeps that read a gate line's flags now stop where the shell does, so a
comment beside a gate may say in words what the gate needs. Supersedes nothing.
Context: ADRs 1718, 1732; traps 13, 25.
Code: `command_words` in `tools/conformance/tests/sandbox_gates.rs`, `state_sections.rs`,
`ratchets.rs` and `bounded.rs`. Test: `a_flag_a_trailing_comment_names_is_not_the_line_s_own` in the
first three, `a_flag_a_trailing_comment_names_is_not_the_walk_s_own` in the fourth.

## 1. The shell's rule, and only it

**The decision.** A sweep that reads `-p`, `--test`, a profile or `walk` off a command line reads the
words before the first word that begins with `#`, which is where the shell ends a command and begins
its comment. Not the first `#` character: a `#` inside a word is an argument (`"$#"`, a URL's
fragment), and a sweep that cut there would read a shorter command than the shell runs. The rule is
the same three lines in each file rather than a shared module, because each integration test is its
own crate and each of the four already carried its own parse of the line.

## 2. The population, derived rather than briefed

The brief named three sweeps, the third as `batch.rs`'s twin. `grep -n '"-p"\|"--test"'` over
`tools/conformance/tests/` names seven readers. `batch.rs`'s `gate_commands` already stopped at a
`#` character, `commands.rs` at ` #` and `workspaces.rs` at the first `#`, and `read_only.rs` reads
only `cargo run … -p conformance --bin`. The three that read the whole line were `sandbox_gates.rs`,
`state_sections.rs` and `ratchets.rs`, the last being `state_sections.rs`'s twin by its own doc
comment. Each kept the *last* `-p` it met, so a comment's flag won. `bounded.rs`'s `state_walks` kept
the *first*, so a comment misled it only where the command lacked the flag, or where the comment named
`--release` on a line that was no walk. It is the fourth.

## 3. Calibrated

Each test plants a gate line whose comment names another package and another target (and, in
`bounded.rs`, a profile). Run before the fix, all four failed by name: `sandbox_gates` read
`pdf-sandbox, --test oracle`, `state_sections` and `ratchets` `pdf-sandbox; --test oracle`, and
`bounded` read `e --test clocked` and took a `cargo test -p conformance` line for a walk. After the
fix all four pass and so does the rest of `cargo test -p conformance` (trap 13).
