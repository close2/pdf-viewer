# 1602 — The confined window asks a link and a named file on its card

Slot 4 of batch ninety-five, 2026-10-19, a host round under the owner's switch to code improvement.
ADR 2040; 2041 and Q497 not used. No ledger row moves: §12.6.4.8's and §12.6.4.3's rows stay
`implemented` and name the confined window's file and tests beside the other windows'.

**Premise.** Held: ADR 1987 kept `--links=` and `--remote-documents=` at `refuse` for want of a
dialogue. **The hypothesis held in its first half**: the flagship's `QuestionCard` is the password
card with the entry taken out, and this window takes it unchanged, on the device and on `--cpu`
(photographed both). **Not in its second**: ADR 1961's operation code is a document's restriction;
these levels are this side's, so nothing new crosses — `Command::Supply` already carries bytes. And
**the drive had no link steps** in any window; steps 90 and 91 are new and judge all four alike.

**Built.** `quorra-confined/asking.rs`: both words at the toolkits' default `ask`, the card answered
by Enter and Escape, one question at a time; a named file read here, parsed by the worker, and the
window's document once the worker says it opened — the prompt's file where it asks for a password,
whose cancel declines it; the offer spent after the one command carrying it (`exchange.rs`). The
submission and embedded-document arms keep their pins with the reason that is left. The residue
of round 1595 is kept and argued (ADR 2040 §3): the core asks no page again at an unchanged target.

**Files.** `crates/viewer-ui/src/bin/quorra-confined.rs`, `quorra-confined/asking.rs` (new),
`quorra-confined/exchange.rs`, `tools/drive-windows.sh` (`link_levels` and its two lists), ADR 2040,
`doc/todo/30-a-native-host.md`, `doc/state-of-play.md`, `doc/conformance/ledger.toml` (two rows'
`code` and `test`).

**Gates.** `rustfmt --check --edition 2024` on the three Rust files: 0. In the round's export:
`-D warnings` clippy `-p viewer-ui --all-targets` 0; `cargo nextest run -p viewer-ui` 210 passed,
2 skipped, 6 new — a plant of the old pins fails 5, and the `refuse` one, the control, passes both;
rustdoc of `tools/state.sh docs`'s three crates 0. The worktree's clippy: exit 101 in `pdf-transform`,
slot 1 mid-edit. `tools/state.sh --round 1602 confined` in the export: exit 0, killed 0. The drive
on Xvfb `:162`, `--step link_levels` in all four windows: 24 works, 0 wrong; HEAD's confined window,
the control: 0 works, 6 wrong (it refuses the words). The confined window's whole list: 24 works,
0 wrong, 10 not offered (script steps). `cargo test -p conformance --no-fail-fast`: 58 binaries, 441 passed, 1 failed (`records`, on 1599's
41-line record; this one is 36). About 4 200 s.

**Unfinished.** `--submissions=` waits for a sender on this side (ADRs 1291, 1527); a question
standing while the worker dies is answered to the new worker, which holds nothing for it.
