# 1532 — The others are behind on the witness, and the returns diminish everywhere but its draw

Slot 5 of batch eighty-four, 2026-10-14: the measure round's step back on the owner's target, to
which the partial-row word yields. ADR 1900; no ADR 1901, no question, no ledger row moved.

**Premise.** Held: `which okular mutool pdftoppm` prints all three, and the clone is there. The
clone is 554 commits behind upstream, so PdfCraft `e4028b3c` was exported and built. The witness
was missing after the 07:34 reboot. The orchestrator restored the original (MD5 `166bb8be…`,
BergPDF `a9f4bac2`), and every figure is of it.
**Hypothesis.** Half held. `quorra`'s witness first frame is 2.04–2.26 s on the owner's display,
against okular's 11.34 s; headless `quorra-transform` is 1.13–1.16 s against `pdftoppm`'s
10.05–10.22 s. "Behind on nothing else" failed three ways. `pdfcraft-cli` reads small pages
2–10 ms faster, because an `ICCBased` photograph is converted per sample (60 M of 97 M
instructions on `WTPDF`). okular turns in 0.027–0.028 s against 0.049–0.078 s (one extra present,
at the instrument's floor). The witness costs 1.84–1.86 GiB against `mutool`'s 278 MiB.
PdfCraft shows "Rendering…" at 0.23 s and the page at 6.95–8.04 s. No toolkit window is behind.

**Verdict.** Diminishing returns on the ordinary documents and on every zoom step. The witness's
first frame has one build left: `Underway` draws for 1.24–1.27 s after a 0.77–0.95 s read,
against the pool's 0.49–0.52 s (slot 3; projected 1.0–1.5 s). Then refactoring (ADR 1900 §5).
**Bug.** On a bare Xvfb, `quorra-gtk bug1815476.pdf` deadlocks before mapping in 5 of 8 starts.
`pdf-sandbox-worker` holds GTK's `dbus-launch` pipes (fds 6–9), and the main thread polls on
them for ever. The window opened on the owner's display, which has a session bus.

**Unfinished.** PdfCraft's zoom on the owner's display: the first sitting timed its placeholder,
and in the second the key reached no window. Xvfb's 2.81–2.96 s stands. The deadlock is not
fixed, and its mechanism is not confirmed in the code.
**Proposed trap.** A window's figure is settled only when the quiet window outlasts the program's
longest silent stage, and the settled picture is looked at. Otherwise "Rendering…" reads as a
0.56 s first frame.

**Files.** ADR 1900; `doc/performance.md` (a new section); this record. Instruments: scratch only.

**Gates.** None owed for the tree. `cargo test -p conformance` in the worktree: exit 0. Two
`--clock --tree 12` holds, both exit 0: one queued 572 s and held 1 399 s at load 1.3–3.5
(headless three rounds, Xvfb two runs, owner's display one run); two queued 267 s and held 244 s
(small pages' stages, and the witness on the owner's display with a 15 s quiet window). HEAD
`--release` exit 0 in 208 s, after a first try killed at the 6 GiB ceiling (exit 137). PdfCraft
`--release` exit 0. Callgrind, small walk, exit 0. Duration 4 900 s.
