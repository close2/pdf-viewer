# 0992 — An import that lints somebody else's lines, and a build error that is not yours

Session 971. Status: **accepted**. Two traps met while working one crate of a tree four other
sessions were editing at the same time. Both are **for `doc/habits.md` or `doc/traps/`**, which
this session does not own (session 972 does), so they are recorded here to be folded in.

Context: `crates/pdf-transform/tests/archive.rs`, `crates/pdf-transform/src/archive/decision.rs`,
`crates/pdf-font/src/standard_metrics.rs` (a sibling's), `doc/conformance/ledger.toml` (a
sibling's).

## 1. Adding a `use` can turn a neighbour's working line into a denied warning

`archive.rs` already used `pdf_syntax::object::Object` in two places by its full path. A new test
of mine wanted the type too, so the obvious move was to add it to the file's imports. That
compiled — and `cargo clippy` then reported **seven** `unused_qualifications` warnings, every one
of them on a line written by an earlier round and correct until my import existed. Under the
project's own gate line, `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`, those
are build failures on code the round did not touch.

The trap is not the lint. The trap is that the blast radius of a `use` is the **whole file**, so
in a tree where several sessions edit adjacent things, an import is a change to lines you are not
allowed to change. Two ways out, and only one of them is right in a shared tree:

- **Right:** write the full path in your own code, as the neighbours already do, and leave the
  file's import list alone.
- **Wrong here:** take clippy's suggestion and rewrite the seven lines. It is a correct patch and
  it puts a round's name on hunks in somebody else's slice, which is exactly what
  `cargo fmt --all` is already forbidden for.

The general shape, and the reason it is worth a line in `doc/habits.md`: **`use`, `cargo fmt
--all` and a shared `mod.rs` are the three ways a round edits files it did not open.** Two of
them are already written down.

## 2. A build error naming a crate you were told not to touch is a sibling mid-edit

Halfway through this round `cargo check -p pdf-transform` failed with

> error: this file contains an unclosed delimiter — crates/pdf-font/src/standard_metrics.rs

which is session 969's file, in a crate this session's instructions forbid it to touch. A few
minutes later the same command succeeded with no action taken. The same thing happened twice more
with `cargo test -p conformance`, which failed on a malformed `test =` array at line 1594 of
`doc/conformance/ledger.toml` and passed on the next run.

**The response is to wait and re-run, and the reasoning is worth stating because the wrong
response is tempting and quiet.** A one-line fix to an unclosed brace looks harmless; it lands in
the middle of another session's edit, and the round that made it will never know why its file came
back different. Two consequences:

- **A gate result is only a fact once it is reproducible**, so a failure whose file is outside the
  round's slice gets re-run before it is reported, and gets reported as *the sibling's* if it
  persists.
- **Never report a green gate that was only ever red for somebody else's reason**, and never
  report a red one without saying whose file it names. Both halves matter: the first is a false
  alarm, the second is a round taking blame it can neither diagnose nor fix.

## 3. A smaller one: the fourth `bool` trips `clippy::pedantic`

`Authorisations` gained its fourth flag this round and `clippy::struct_excessive_bools` fired.
The lint's remedy — a set — is exactly what the type's own comment declines, because a set lets a
caller hold a loss no `match` arm answers, where a field makes a new loss a compile error at every
site that has to answer it. So the answer was `#[expect(…, reason = …)]` naming that argument,
which is the shape `CLAUDE.md` principle 1 asks for: a shortcut is fine when its cost is written
down, and here the "shortcut" is refusing the lint's advice with the reason beside it.

Worth knowing in advance rather than discovering at the gate: **three bools is the threshold, so
the round that adds the fourth pays for a decision three earlier rounds made.**
