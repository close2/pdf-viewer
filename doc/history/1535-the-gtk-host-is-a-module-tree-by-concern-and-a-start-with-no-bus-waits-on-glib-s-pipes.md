# 1535 — The GTK host is a module tree by concern, and a start with no bus waits on GLib's pipes

Slot 2 of batch eighty-five, 2026-10-14, a host round on the owner's switch to code improvement.
ADRs 1906 and 1907; Q430. No ledger row moved: neither change touches a clause.

**Premise.** Held: `host.rs` was 7 955 lines; `pdf-sandbox/src/lib.rs` spawns the worker with
`Command`, piped stdin and stdout, inherited stderr and no `pre_exec`; round 1532's fd table.
**Hypothesis.** Held in its first half: the leak is GLib's spawn reaching Rust's. `strace -f`
shows `pipe2(…, 0)` on GTK's thread from `gdbusaddress.c`'s `g_spawn_command_line_sync`, and the
worker started inside that window holds 6–10 (ADR 1907 §1). Rust's own pipes are close-on-exec,
and GLib's child closes what it did not mean to pass, so the reverse does not happen.

**Built.** (a) `viewer-gtk`'s host is a root and eighteen children named as slot 3 named Qt's,
plus `rich_text`, the one concern Qt has no file for (ADR 1906). (b) Drive step 84,
`bare_session_start`: eight starts on an X server of its own with no session bus, each given 10 s.
**Not built.** Every fix needs one `unsafe` call in or beside the decoder's process, which
principle 3 forbids. The owner's word is Q430. A scratch close at the worker's first statement
took the hang from 7 of 16 starts to 0 of 16.

**Proof that (a) moved nothing.** One export of HEAD `512af7e5` and one of HEAD plus
`crates/viewer-gtk`. Each was a release build, and `quorra-gtk`'s drive ran whole in each. Before:
87 works, 4 not offered. After: the same 91 verdicts, step for step. Every pixel count and title is
equal. The only differences are a loopback port, a loading bar's percentage, a page's finish time
and a transition's frame count, all timings. `py/same.py` finds each of HEAD's 232 items once
(ADR 1906 §2).
`cargo doc --document-private-items`: HEAD's eleven warnings, no others.

**Unfinished.** Q430's answer. Step 84 has run only in `quorra-gtk`: `quorra`, `quorra-qt` and
the confined window take it at the merge's drive. With the close, `pdf-view-worker` and
`pdf-script-worker` would owe it too.
**Proposed trap.** A race measured under the lock is measured with the lock's environment, and
`RAYON_NUM_THREADS=4` made this one a twelfth as likely. The window under test runs without it.

**Gates.** `rustfmt --check` on the 20 files: clean. `RUSTFLAGS="-D warnings" cargo clippy -p
viewer-gtk --all-targets`: exit 0. `cargo nextest run -p viewer-gtk`: 16 of 16, in the worktree
and the export. Drive, before and after: exit 0 each. Step 84: HEAD `wrong`, 5 of 8; scratch close
`works`, 8 of 8. `cargo test -p conformance --no-fail-fast`: one failure,
`state_sections`, on slot 5's `pdf-model/src/image/cut.rs`. `tools/batch.sh check`: exit 0.
`launch_path` cannot move: nothing depends on `viewer-gtk`. The `confined` tests are not owed,
because the spawn is unchanged.
