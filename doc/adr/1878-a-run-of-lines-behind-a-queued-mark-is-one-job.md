# 1878 — A run of lines behind a queued mark is one job, and a line instance is one write

Session 1520. Status: **accepted** and **built**. The encode half of what ADR 1877 left a zoom step
on the witness. Context: raster's ADRs 0023, 0097; this tree's ADRs 1409, 1835, 1877. Code:
`raster/crates/raster-gpu/src/encode/parallel.rs` (`Place::Lines`, `Job::lines`, `extend_lines`,
`line_run`), `encode/parallel/commit.rs` (`enqueue_line`, `commit_lines`, `drain`),
`encode/instance.rs` (`QueuedLine`, `write_line_instance`), `encode.rs` (`Encoder::queued_lines`).
Held by raster's `tests/encode_threads.rs` (byte equality across thread counts) and `tests/
line_lane.rs`. No ledger row moves.

## 1. What the instruction count said

Once the scene was kept (ADR 1877), the witness's step was raster's encode: 573–680 ms for
4 408 406 commands. Callgrind over two encodes (one exploring run, the encode's own functions):
about 1 340 instructions a line. Every line behind a queued mark — a fill whose coverage the
fan-out makes — was a `Job` of its own (ADR 1409's rule for geometry-free marks): a record of
several hundred bytes copied on `enqueue`, again on the drain's `take`, again into the commit's
closure — 1.72 G of `memcpy` from those three sites — and each line's commit asked `drain_queue`
again. Writing the instance was fifteen `extend_from_slice` calls of four bytes, 195 instructions.

## 2. Decision

**A line queued behind a mark joins the run the last queued job is, or starts one**: the instance
and its draw go into `Encoder::queued_lines`, and the job is `Place::Lines { start, end }`, a range
of it. The commit writes the run's instances in order after one `drain_queue`, which is the order the
one-threaded walk writes them, so every byte is that walk's (`tests/encode_threads.rs`, unchanged).
The queue's bound is kept a line at a time — a queued line holds its own size — and a drain the bound
forces writes the line straight after, where the walk with nothing queued writes it.

**A line instance is one write of its sixty bytes**, assembled in an array whose length the compiler
holds equal to `LINE_INSTANCE_STRIDE`.

## 3. What it moved

Callgrind, two encodes of the witness (the fit and 1.25×, `Cpu`): `Device::encode_scene` 11.76 →
6.47 G instructions, **1 334 → 734 a command**; `enqueue`'s and the drain's `memcpy` gone from the
profile's head. In the clock hold of ADR 1877 §3 the encode of a step is 206–325 ms where the walk
before this and ADR 1877 took 357–459 (HEAD, whose scene was also rebuilt); in the exploring runs
between the two changes, with the scene already kept, 1 144–1 566 → 249–580 ms, at loads that
differed.

What is left of a line's 734 instructions is the walk itself — the command's dispatch, `pen`'s
width resolution, the clip and the ink box — and a run of lines is as parallel as any: each line's
instance is a pure function of the command and the view. Encoding a run on the fan-out's threads and
appending it in order is the next step on this side, before ADR 1877 §5's resident instances.

What it costs: one more vector beside the queue, and a commit loop that knows one job kind by name.
