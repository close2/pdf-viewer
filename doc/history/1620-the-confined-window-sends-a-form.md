# 1620 — The confined window sends a form

Slot 6 of batch ninety-seven, 2026-10-20, a host round under the owner's switch to code improvement.
ADR 2076; 2077 and Q515 not used. No ledger row moves: §12.7.6.2's row stays `implemented` and names
the confined window's sending module and test beside the other windows'.

**Premise.** Held: round 1602 pinned `--submissions=` at `refuse` for want of a sender on this side.
**The hypothesis held in two of its three parts**: the card takes `may_submit`'s question unchanged,
and the transmission is `Submitter::send`, the thread every window puts `transmit` on, polled at
`LOOK` since this window has no proxy. **Not in its third**: `Command::Respond` did not cross — the
wire named it `Uncarried` — so a command discriminant (43) is added and the greeting moves to
`PDFVCF18`. And **the drive had no submission step** in any window; steps 92 and 93 are new.

**Built.** `quorra-confined/sending.rs`: the word at the toolkits' default `ask`, the request the
worker composed sent from this side, the answer collected on the loop's look and handed back as
`Command::Respond`, which the worker imports. An import named by an absolute URI is fetched under the
same level, since two levels for one act in one window is what the old sentence would have become;
a PDF answer is kept and said, not opened, because this window holds one document; a policy copy is
still declined, for that reason now rather than "no network" (ADR 2076). `asking.rs` gains the two
questions; the drive's `submission_levels` runs both acts at five levels with its own loopback server.

**Files.** `crates/viewer-ui/src/bin/quorra-confined.rs`, `quorra-confined/asking.rs`,
`quorra-confined/sending.rs` (new), `crates/viewer-confined/src/protocol.rs`, `tools/drive-windows.sh`
(`submission_levels` and its two lists), `doc/ui-boundary.md` (the greeting), ADR 2076,
`doc/todo/30-a-native-host.md`, `doc/state-of-play.md`, `doc/conformance/ledger.toml` (§12.7.6.2's
`code`, `test` and one sentence of its note).

**Gates.** `rustfmt --check --edition 2024` on the four Rust files: 0. `cargo fmt --check -p viewer-ui
-p viewer-confined` 0; `-D warnings` clippy `-p viewer-confined --all-targets` 0 and `-p viewer-ui
--all-targets` 0 (worktree); `cargo nextest run -p viewer-confined -p viewer-ui` 252 passed, 2
skipped, 4 new in the window and the round trip extended — the old pin planted fails all 4, the
`refuse` half the control. `cargo test -p conformance --test greetings` 2 passed; `cargo test -p
conformance --no-fail-fast` 59 binaries, 443 passed, 0 failed. In the export: `tools/state.sh --round
1620 confined` exit 0, killed 0; rustdoc of `docs`'s three crates 0. The drive on Xvfb `:170`,
`--step submission_levels`, four windows: 40 works, 0 wrong; HEAD's confined window, the control: 0
works, 10 wrong (it refuses the word); the rebuilt confined window with `link_levels`: 16 works, 0
wrong. About 3 100 s.

**Unfinished.** An answer arriving after the worker died goes into the form as reopened (ADR 2076
§3). `viewer-confined`'s rustdoc fails on 11 private links at HEAD and here alike, outside `docs`.
