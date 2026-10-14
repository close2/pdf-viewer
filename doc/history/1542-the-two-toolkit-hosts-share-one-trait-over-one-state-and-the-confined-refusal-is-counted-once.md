# 1542 — The two toolkit hosts share one trait over one state, and the confined refusal is counted once

Slot 3 of batch eighty-six, 2026-10-14, a host round of the owner's switch to code improvement.
ADRs 1920 (the shared half) and 1921 (step 82). No question, and no ledger row moves.

**Premise.** (a) and (b) held: 31 methods are identical in both hosts and 13 are 81–96 % alike,
and step 82 reads `wrong` for `quorra-confined` at HEAD. (c) is not open, because there is no
`A430`. **Hypothesis.** Failed. The worker refuses once at each of the five notches past the bound,
not at every frame (ADR 1892 §2.4). The drive counted each refusal twice, the window's sentence and
the events trace's echo of it. The pointer moves sent the worker nothing (ADR 1921).

**Built.** (a) `viewer_host::native_window` holds `WindowState` (19 fields), `Showing`,
`DOCUMENT` and `NativeWindow`. The trait has 32 provided methods (28 identical, 4 near-identical)
and 4 required ones (`say`, `dispatch`, `repaint`, `later_open_the_next`); each host forwards those
4 to its own. `run`, `fetch_policies`, `fmt`, `react`'s match and 8 near-identical methods stay
(ADR 1920 §3). (b) Step 82 counts the window's own sentence. It is `wrong` where one size is
refused twice or a pointer move is refused. On HEAD's log: as found `works 5/5/0`, with a planted
repeat `wrong 6/5/0`, with a planted late refusal `wrong`. Unfinished: past the bound the confined
window still refuses (ADR 1921 §3).

**Proof that nothing moved.** Token for token, comments included, once `.state` is taken out:
221 of GTK's functions and 356 of Qt's are HEAD's. The rest are `open`'s literal, three rustfmt
commas, and Qt's forwarding `find`, which `cxx` needs. Each provided method is both hosts' HEAD
code, but for `self.state()`, a `&mut` receiver, `repaint()`, `documents::label` for the hosts'
identical `named`, and a directory GTK's `write_extracted` now clones. Of 32 fields tried private
only `warned` compiled. `quorra-confined` and the workers are byte-identical in both exports.
The drive of the four windows: 298 verdicts each, 285 works and 3 wrong at HEAD, 286 and 2 after.
Besides step 82, Qt's 82-retained-page and `quorra`'s 83-transition-frames differ, and each reads
`works` 3 of 3 times run again in both. 265 of 329 photographs are pixel-identical; 64 differ in
status-line paths, ports or percentages, a caret, or a shot's moment (GTK's 14 and 18 rerun match).

**Files.** `viewer-host/src/{lib,native_window}.rs`; `viewer-gtk/src/{access,host}.rs`,
`bin/quorra-gtk.rs` and 16 of `host/`; `viewer-qt/src/{access,host}.rs`, `bin/quorra-qt.rs`, 17 of
`host/`; `tools/drive-windows.sh` (step 82); ADRs 1920–1921; `doc/crate-map.md`; `doc/todo/30`.

**Gates.** rustfmt `--check` on all 41 files: clean. `clippy --all-targets` on the three crates:
exit 0, no warning in them; `-D warnings` stops in a sibling's `pdf-font` build script. nextest of
the three: 314 passed, exit 0; `-p viewer-confined`: 104 passed, exit 0. `test -p conformance`:
436 passed, exit 0, after it caught three doc links to moved fields. Drives (`--long --tree 6`):
1 242 s and 1 194 s, reruns, `state.sh launch` (`--clock`, 186 s, 0 outside): exit 0. 9 840 s.
