# 1481 — A carriage return and line feed are one break, and GTK's combo box says its arrow

Slot 2 of batch seventy-six, 2026-10-09, a host round. ADRs 1798 and 1799; Q377 not used. No ledger
row moved; §12.5.6.2's `implemented` note gains the pair's reading and a test. No `partial` row was
worked: none is this slot's.

**Premise.** Held in the code: `chrome.rs` split on each byte at 3027, 3076 and 3239, so `"a\r\nb"`
drew an empty line between its paragraphs, and `viewer_host::popup::thread` did the same for Qt.
`controls.rs:508` built the drop-down. **Hypothesis, half held.** "No sentence binds" was false:
§12.5.6.2 states "a CARRIAGE RETURN (0Dh) shall be used and not, for example, a LINE FEED". It binds
the writer and names no pair, so the pair being one break is still a choice, on §3.23 and §7.2.3
(ADR 1798). The popover hypothesis held. GTK 4.22's source shows its list moving `popup_selection`
inside the key press, while the drop-down copies it only on `row_activated` (ADR 1799).

**Built.** `viewer_host::popup::paragraphs` is the one split, used by `quorra`'s window, replies and
caret places, by `thread`, and by Qt's `escape_into`, which used to drop a lone carriage return (the
clause's own separator). `quorra`'s caret steps over the pair as one character. GTK's drop-down takes
an arrow's option at once and sends it with the arrows held. The editable combo box's list does the
same, and the Enter that closes either list sends nothing more. Drive step `74-combo-arrows` covers it.

**Found.** This program writes a line feed into a note's `/Contents`, where §12.5.6.2 asks for a
carriage return: `quorra`'s Enter, and the toolkits' editors. This is listed in `doc/todo/30` and
ADR 1798 §3, and not started. Editing `tools/drive-windows.sh` while it ran made bash read on from a
shifted offset, so three note steps ran in the confined window that its list never runs.

**Gates.** `rustfmt --check --edition 2024` on my five Rust files: exit 0; `bash -n` on the drive:
exit 0. `cargo clippy -D warnings -p viewer-host -p viewer-ui -p viewer-gtk --all-targets`: exit 0.
`cargo nextest run` on the same three: exit 0, 418 passed and 2 skipped. Plant (trap 13): with the
pair as two separators, the splitter test and the window test fail, and the caret test, which does
not read the split, passes. `cargo test -p conformance`: 430 passed and 0 failed (one
earlier run failed only slot 6's `arms` test, while that slot was unfinished). Tier 2 was `tools/drive-windows.sh` whole, as a `--tree 12` walk on release
binaries copied to `scratchpad/r1481/bin`. The first run exited 2, from the mid-run edit above.
The second exited 0 with 262 works, 2 wrong and 6 not offered; step 74 works in the three windows that
run scripts and is not offered in the confined one. The 2 wrong are GTK steps 20 and 24: every placed
control was missing after the third typed value. They passed in 4 of 5 runs of this build and 3 of 3
of HEAD's, so the failure is intermittent and not traced. Control: HEAD's `quorra-gtk` logs `C 1 Two
keyDown false` at step 74, at the Enter. Lock: the first ask was stopped after 1 859.5 s queued
(`lane=-`); the drives waited 575.7 s and 0.0 s and held 980.4 s and 916.8 s. `launch_path` is not
owed, because no open path changed. Duration 7 000 s.
