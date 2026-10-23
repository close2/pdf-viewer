# 1711 — The confined wire splits by concern

Session 1711, slot 8 of batch 109, the standing code-quality slot; ADR 2256; no question.
Started 2026-10-23T07:59:37+02:00; finished 08:37; 2240 s. No bug fixed; no model change; ledger statuses unchanged.

**Choice.** `viewer-confined/src/protocol.rs` (7 951 lines, the largest source file) is split into
eleven children by concern plus five test files: `wire`, `frame`, `error`, `command`, `event`,
`query`, `answer`, `view_fields`, `edit`, `marks`, `payload`, `measurement`, and `tests/{commands,
events_queries,panel_answers,frames,refusals}`. The root is 145 lines: documentation with the
module table, the greeting, the frame kinds and the re-exports. `read_quads` moved to `wire`.

**Proof.** Tokens of non-comment lines, before and after, match once imports, visibility and
module declarations are removed: only `mod tests {`/`}` are old-only and ten trailing commas
(rustfmt after a dedent) new-only. 31 tests before, 31 after. Each added `pub(super)` was removed
in turn and compiled (97 tried, 6 narrowed). Ledger row 12.6.4.3 re-pointed to `command.rs` and
`event.rs`; `crate-map.md` and the teaching walk redrawn.

**Gates.** `cargo test -p viewer-confined`: 71 + 46 passed, exit 0 (81 s); fmt and
warnings-denied clippy (all targets) exit 0; private rustdoc 0 unresolved links; fuzz workspace
`cargo +nightly check` with `-D warnings` exit 0 (47 s). `cargo test -p conformance`: flags, greetings, workspaces, ledger_notes, spelled_ordinals and
round_numbers pass; the ledger test fails on sibling rows naming ADRs 2252 and 2253 (no file yet)
and `records` on 1710's Gates paragraph, neither mine. No arm owed: the arms' binaries do not link this crate's wire in their render path.

**The gate that moved with the files.** `tools/conformance/src/flags.rs` skipped only inline
`#[cfg(test)]` modules, so a test kept in `protocol/tests/` read as the program; it now also skips
the files of a `#[cfg(test)] mod name;` (ADR 2256).

**Found, not fixed.** `decode_answer`'s doc ("Reads one answer. # Errors ...") sits above
`encode_traced`'s in `protocol/measurement.rs`, so `decode_answer` (in `answer.rs`) has no doc and
`encode_traced` has two. Carried over verbatim; a one-line move for a bug round.
