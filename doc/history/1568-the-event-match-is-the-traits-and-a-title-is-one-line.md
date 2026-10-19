# 1568 — The event match is the trait's, and a title is one line

Slot 5 of batch ninety, 2026-10-15, a host refactoring round under the owner's switch to code
improvement. ADRs 1972 and 1973; no question; no ledger row moves.

**Premise.** It held: both `react`s carried `too_many_lines`, ADR 1920 §3 left the match and eight
methods twice, and `PDF20_AN001-BPC.pdf`'s item `…001:  \n Black Point Compensation` reached all
four titles with its break. **Hypothesis.** It held but for one difference, written down: Qt no
longer flags a frame for a window the drawing abandoned (ADR 1972 §2).

**Built.** `NativeWindow::react` is the one match, every arm one line (habit 103); both `expect`s
are gone. A toolkit's own answer is a required method its `impl` forwards; Qt's `ask` and three
`NeedsFile` arms became methods. Shared too: `turned`, `restate`, `fetch_policies`,
`take_the_drawn`, `answered`, `take_the_windows`, `send_form`, `start_policy_fetch`,
`obey_the_catalog`, `take_the_opening`, `mind_the_opening`, `convert`, the password's supply; four
fields moved into `WindowState`. `documents::caption` and `one_line` put a title on one line in all
four windows, and the confined refusal title takes its first line (ADR 1973): HEAD's four titles
held `\n`, these hold none. `grep -c "fn "` over `host/*.rs`: GTK 168 → 156, Qt 209 → 200; trait
methods 41 → 88; lines GTK 9 891 → 9 475, Qt 8 507 → 8 138, the module 994 → 1 703. The report sweep
counts a `NativeWindow` implementor as asking where `viewer-host` does; planted out of the trait, it
names both toolkit crates. Unfinished: `open_document` stays each host's, a required method — it
names three host-only fields and slot 2 was writing into it; `run` stays (ADR 1920 §3).

**Proof that nothing moved.** Each moved body is a host's HEAD code token for token, comments set
aside and `self.state()` read as `self.state`, but for the hooks, `pump`/`drain`, `refresh` as
`repaint` and split borrows (a script the round left in its scratch). The drive of the four windows, HEAD's
export against HEAD plus this patch alone (siblings' hunks filtered out): HEAD 293 works, 3 wrong,
10 not offered; after 296 works, 0 wrong, 10 not offered. HEAD's three wrongs (GTK 25, 45, 82) read
`works` run again. 270 of 335 photographs are pixel-identical; of the seven differing most, GTK's 18,
45 and 82-mid match HEAD's taken again, and GTK's 66-refused and `quorra`'s 48 are a shot's moment
(the policy log has HEAD's events in HEAD's order, its first frame 32 ms sooner).

**Files.** `viewer-host`'s `native_window.rs`, `documents.rs`, report sweep; `viewer-gtk`'s and
`viewer-qt`'s `host.rs`, program, 10 of `host/`; `quorra-confined.rs`, `quorra/dispatch.rs`; the map.

**Gates.** In the export: rustfmt `--check` on the 29 files clean; `-D warnings` clippy
`--all-targets` on `viewer-host`, `-gtk`, `-qt`, `-ui` exit 0; their nextest 516 passed, 3 skipped,
exit 0; the docs gate's `cargo doc` exit 0. `cargo test -p conformance` in the worktree: exit 0,
436 passed. `tools/state.sh hosts`: 37 of 45, 36 of 36. Drives 1 274 s,
1 178 s. 7 400 s.
