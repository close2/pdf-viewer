# 1780 — A test that needs a draw in progress holds it open rather than timing it

Session 1473. Status: **accepted** and **built**. Closes `doc/todo/67`; the evidence ADR 0650 rests on.
Code: `crates/viewer-confined/tests/confined.rs` (`HeldOpen`, `REACHES_THE_MARK`, the test
`a_host_drawing_marks_that_will_not_finish_interrupts_its_own_draw`).

## What was wrong

The test raised its `pdf_render::Interrupt` after waiting a fixed time and asserted first that the
host's draw had not finished in that time. The draw is the amplification fixture's ten thousand
page-covering fills, and through `CpuRasterizer::new()`, as `viewer_host::drawing` makes it, it
finished unheld in **0.4 to 0.8 s** in release and **1.9 to 2.4 s** in the `dev` profile the suite
builds (three runs each of `examples/confined_cancel --marks --finish`, 2026-10-08, load average 10
to 24, 24 cores). ADR 0650 priced it at 27.6 s. A wait chosen against that draw is a race, which a
quicker machine or a quicker rasteriser loses; a re-tuned constant decays the same way again.

## Decision

**A test that needs a draw in progress stands inside the draw's command loop and holds it there.**
The one producer `render-cpu` calls in the middle of that loop is `pdf_render::ImageAtDeviceScale`
(`ImageSource::at` from `draw_image`), so the test puts a page-covering image of its own, backed by
`HeldOpen`, in front of the worker's marks. Its `samples` tells the test it has been reached, then
waits on a gate the test owns. The test raises the interrupt only once the mark is reached and the
draw has not answered, then opens the gate: the rasteriser finishes the mark and refuses at the next
command, with all ten thousand fills still to come. That holds on any machine and at any strip count,
since every strip that draws the mark waits on the same gate (run alone pinned to one core, to eight
and unpinned: 19.3, 43.0 and 25.7 ms from the gate opening to the refusal, the mark's own draw
included).

**Calibrated by breaking it** (trap 13): with `.interruptible(...)` left off the host's rasteriser,
the test fails on "the draw came back with 4194000 B of pixels rather than a refusal".

## Consequences

- Ten `cargo test -p viewer-confined` runs in a row pass, 101 tests each.
- The worker-side cancel test still waits `UNFINISHED` (2 s) before its kill, against a draw of
  **16.3 s** in `dev` and **6.9 s** in release (measured the same day). That draw happens inside the
  confined worker, which the test reaches only through the document, so a held mark has no way in;
  its comment carries the price, and a round that sees the margin shrink owes it a deeper fixture.
- `examples/confined_cancel --marks` still draws for 250 ms before raising, against the 0.4 s
  release draw; it prints whether the draw was interrupted or finished, so it cannot claim the first
  falsely, and it stays a demonstration rather than a gate.
