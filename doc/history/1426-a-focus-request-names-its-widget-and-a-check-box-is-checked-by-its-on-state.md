# 1426 — A focus request names its widget, and a check box is checked by its on state

Slot 1 of batch sixty-seven, 2026-10-08, a script round. ADRs 1688 and 1689. No ledger row moved:
the members are Adobe's reference, not a clause; §12.7.5.2.3 and Table 166's `/P` were read, not
touched in status. No question written.

**Premise.** Held in part. `bridge.rs` had two `NotBridged` sites, lines 1393 and 1909, and
`setFocus` was refused through a widget after the first. The census premise did not hold as
briefed: runs reach one listed member, `app.setInterval`, because the column walks no Table 197
event and 9 177 of its 9 317 throws are `ReferenceError`s that stop a script early. The static
census was read beside it. Found beside the focus: `carry_out_focus_requests` read `/P` through
`get_key`, so no focus request had ever turned a page.

**Focus (ADR 1688).** `ScriptEdit::Focus` carries the widget's index and the wire is version 7.
The view state checks the index and hands on a `FocusRequest`. The host focuses that widget on its
own page. Planted back: `widgets.first()` fails the viewer test with object 6, and the old `/P`
read fails it with page 0.

**Census and bridge (ADR 1689).** The column prints a `not bridged` line for every listed member,
zeros included; 1 of 52 listed members was reached. The static census over the same 90 763 files
found `getAnnots` in 34 documents, `app.beep` in 24, `checkThisBox` in 20 and `isBoxChecked` in 14.
The two check box methods are bridged on a widget's §12.7.5.2.3 on state, which now crosses to the
realm. The column now reads 1 of 50. The tail is ranked in ADR 1689 section 4: `getAnnots`, then
`app.beep`, then the timers, which need a host that ticks outside a presentation. Class 6 is
unbuilt.

**Gates.** `rustfmt --check` on the 18 changed `.rs` files: exit 0. `-D warnings` clippy on
`pdf-script` (with and without `engine`), `pdf-script-worker` (with and without `engine`),
`pdf-model` and `viewer-core`, all targets: exit 0. nextest: `pdf-script --features engine`
102 passed, 3 skipped; `pdf-script-worker --features engine` 25 passed, 1 skipped; `pdf-model`
1 985 passed, 19 skipped; `viewer-core` 343 passed, 2 skipped. `cargo test -p conformance
--no-fail-fast`: exit 0. Behind the lock (`--tree 6`), the Tier 1 column ran twice, each exit 0.
Both runs gave 21 101 runs: 11 776 finished, 9 317 threw, 8 unparsed, 0 over a budget, so
`HELD_THREW` and `HELD_RUNS` are unchanged. The second run held for 148 s after a 660 s wait,
peak 1.34 GiB. The worker column gave the same five figures with 0 lost and 0 `SIGSYS` (170 s
held). The `pdf-model` corpus: exit 0, every ratchet at slack 0. Tier 0's `script_corpus`: exit 0,
356 held and 0 moved. `javascript_census`: exit 0, 155 s. Duration 4 250 s.
