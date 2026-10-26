# 1759 — The confined window's `Host` is continued in five modules by concern

Session 1759, slot 8 of batch 115, the standing Sonnet 5.5 code-quality slot; ADR 2352; ADR 2353 unused; no question.
No bug fixed; no model change; ledger statuses unchanged.

**Choice.** `tools/state.sh gods` named `Host` in `crates/viewer-ui/src/bin/quorra-confined.rs` (one impl block of
1 306 lines, 32 methods) after `Interpreter`. It is not mostly one `match` (`event` is one, about 300 lines, and
stays whole), so `Host` was taken and `Vfs` left. The methods are now in `quorra-confined/worker.rs`, `events.rs`,
`frames.rs`, `window.rs` and `input.rs`, one `impl Host` each; the command line and the tests moved to
`arguments.rs` and `tests.rs`. The root fell from 2 684 to 695 lines; the hypothesis of under 600 did not hold
(the type, constructor, `ApplicationHandler` and module documentation are what remains).

**Proof.** Multiset of non-comment lines, indentation and `pub(crate) ` stripped, HEAD's file against the root and
its seven new children: no code line lost; only `use`, `mod`, `impl Host {` and brace lines added, and the root's
unused imports and the empty impl block removed. The 18 tests of the file are the 18 in `tests.rs`. The ledger rows
naming `quorra-confined.rs::` for three tests now name `quorra-confined/tests.rs::`; `doc/crate-map.md` and
`doc/todo/74` name the modules. No teaching diagram names a method that moved.

**Gates.** `cargo fmt --check -p viewer-ui` exit 0 and `RUSTFLAGS="-D warnings" cargo clippy -p viewer-ui
--all-targets` exit 0; `cargo test -p viewer-ui --no-fail-fast` exit 0 (45 tests in the confined binary). The drive of
the confined window under Xvfb, an export of this tree against HEAD's binaries: 49 works, 1 wrong, 1 not offered on
both, the same verdict on all 51 steps (the `wrong` is step 66 on both; the export build took 369 s, a drive 210 s
and 221 s); the Xvfb servers ended. `cargo tree -i viewer-ui` prints no dependent, so the six arms' binaries do not
link the crate and need no re-run. `cargo test -p conformance --no-fail-fast` exit 0 (135 s of the lock after 1 431 s queued behind a sibling; no red).

**Found, not fixed.** `Host::event` is still about 300 lines under an `#[expect(clippy::too_many_lines)]` that is
true of its exhaustive match; `Host::new` lists 45 fields, which a later pass could group (the title's, the
asking cards', the levels'). `a_host`'s comment in `tests.rs` says "this file added" of a file it no longer is.
Step 66 (`signature-policy`) reads `wrong` in the confined window on HEAD as well.
