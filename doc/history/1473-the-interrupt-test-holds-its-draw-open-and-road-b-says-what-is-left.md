# 1473 — The interrupt test holds its draw open, and road B says what is left

Slot 6 of batch seventy-four, 2026-10-08, a loop round. ADR 1780; ADR 1781 and Q369 not used. No
ledger row moved: the contract is a test and a todo's status, and no clause is reached.

**Premise.** Held. `HOST_UNFINISHED` stood at lines 1328, 1348, 1410 and 1411 and read 200 ms;
`doc/todo/67` priced the draw at 27.6 s.

**Hypothesis.** Held. Unheld, the ten thousand fills finish in 0.4 to 0.8 s in release and 1.9 to
2.4 s in `dev` (`examples/confined_cancel --marks --finish`, three runs each, load average 10 to 24),
so the 200 ms wait passed on a margin of two in release, which a faster machine or rasteriser
closes.

**Built.** The test puts `HeldOpen`, a page-covering `ImageAtDeviceScale` the test owns, in front of
the worker's marks. Its `samples` says it was reached and waits on a gate; the test raises the
interrupt there, opens the gate, and the draw refuses at the next command with 10 000 marks to come.
The test's comment states the measurement. `UNFINISHED`'s comment now states the worker-side price,
16.3 s in `dev` and 6.9 s in release, and why that wait cannot be held. `doc/todo/67` is deleted
with its index row, since nothing outside `doc/adr/` cited it (ADR 1780). `doc/todo/15`'s status
says what is: one window on the boundary, three windows still in process, the real-adapter figures
the owner's. Its index row lost "the owner's warn-then-abort" as owed, since ADR 0729 built it.

**Gates.** `rustfmt --check --edition 2024` on `confined.rs`: exit 0. `cargo clippy -D warnings -p
viewer-confined --all-targets`: exit 0. `cargo nextest run -p viewer-confined`: 101 passed, 1
skipped. `cargo test -p viewer-confined` ten times in a row: exit 0 each, 101 passed, 0 failed.
The test alone, pinned to core 0, to cores 0 to 7 and unpinned: passed, 19.3, 43.0 and 25.7 ms
from the gate to the refusal. Calibration: with `.interruptible` dropped the test fails, "the draw
came back with 4194000 B of pixels rather than a refusal"; restored, it passes. `cargo test -p
conformance`: 427 passed, 0 failed. `awkward_classes` not run: the worker is unchanged. The core,
`selection_census` and `accessibility_census`, which `tools/round.sh loop` names, were not run:
only a test target changed. Duration 1 400 s.
