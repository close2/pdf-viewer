# 2668 — The clip test, run alone on the RX 580 under RADV's hang mode, does not hang it, and the guards stand

Session 1919, a single round (pixels + instruments). Status: **accepted**; the supervised run is
**built** as `tools/hang-capture.sh` and **taken once**; **the cause of the 14:02:33 hang is still not
found**, and Q777 asks the owner for the next run. Context: ADR 2663 (both halves of device
selection), ADR 2664 (the lock around each device section), ADR 2665 (the resets read from the kernel,
the hang marker, the drafted upstream report), the owner's A776 (the licence for this run and its
conditions). It cites no clause and moves no ledger row.

## 1. The run, as A776 allowed it

`tools/hang-capture.sh` is the owner's conditions written as one script, so that nothing in them
depends on a person's timing at a shell:

1. the test binary is built first, outside any lock;
2. the script takes the shared lock, `/run/lock/gpu.lock`, and keeps it until it ends: it runs itself
   a second time under the machine's `with-gpu-lock`, so the wrapper's lock covers the whole
   procedure below and not the test alone;
3. inside, the test runs `--exact` on RADV (`WGPU_BACKEND=vulkan` and the radeon ICD, ADR 2663) with
   `RADV_DEBUG=hang`, under a wall of 60 s, with its own section hook pointed at a private lock file —
   the hook would otherwise wait for ever on the lock the script holds — whose hang marker (ADR 2665
   §3) is cleared first;
4. it reads `journalctl -k` since the run began; on a ring timeout or a reset it waits, still inside
   the lock, for `GPU reset(n) succeeded`, and then, hang or none, for `vulkaninfo --summary` on RADV
   to exit 0, for at most ten minutes in all;
5. only then does the script end, and the wrapper's lock go with it.

It is a script in the tree, and the test is not simply given to `with-gpu-lock`, because the
wrapper releases the lock when its command ends, which for a test that hung the card is before the
kernel has finished resetting it — and the reset is the moment the other user's work has to be kept
off the card. Given the whole procedure instead, the wrapper holds the lock across the reset and the
probe. The run below was taken by the script's first form, which held the same exclusive `flock` of the
same file on a descriptor of its own; ADR 1662's gate (`bounded.rs`, every lock a tool takes is taken
by a wrapper) refused a bare `flock` under `tools/`, and the machine's own wrapper is the one this lock
has, so the script now takes it that way, with the same hold. It is argued as a device-only tool in
the sense of the CLAUDE.md machine rule: it wraps no build, no gate and no suite, and the binary it is
given was built before it was called.

The run, 2026-11-03 (times UTC, from the script's own log):

| step | time |
|---|---|
| lock asked for | 23:56:55.910 |
| lock held (waited 11 ms) | 23:56:55.921 |
| test start | 23:56:55.939 |
| test end, exit 0 (`1 passed`, 0.81 s) | 23:56:56.777 |
| kernel log since the start | no entry |
| `vulkaninfo --summary` on RADV, exit 0 (`AMD Radeon RX 580 Series (RADV POLARIS10)`, `radv`) | 23:56:56.892 |
| lock released | 23:56:56.946 |

The lock was held 1.0 s. No reset, no ring timeout, no fault; RADV wrote no hang dump,
because RADV writes one only on a hang it detects. The private lock's log shows two bring-ups of
type `DiscreteGpu` and two frames (176 ms and 64 ms: the 1× and the 2× of the test, the 2× being
the frame that hung at 14:02:33), so the run drew on the card and drew the frame in question.

## 2. What a pass here says, and what it does not

Two things differ between this run and the one that hung, not one, and the pass cannot say which of
them removed the hang:

- **Alone, not in a suite.** At 14:02:33 the clip test ran inside a parallel `nextest` of three
  crates, with other test processes holding devices on the card; here no other process of ours had
  a device open, and the other user's could not submit while the lock was held.
- **Under the hang mode, which serialises.** RADV printed `Enabled debug options: syncshaders,
  hang`: `RADV_DEBUG=hang` turns on `syncshaders`, which waits for every draw and dispatch to finish
  before the next starts and waits for every submission before returning. A hang that needs two
  pieces of work in flight together on the card, inside one command buffer or across two, does
  not occur under it.

So ADR 2665 §1's last reading — every reset happened inside a parallel suite, and the clock gates,
one device process at a time, have reset nothing in 355 holds — gains one more observation of the
same kind and no proof. Q776 had written that a pass alone would "itself" be "the finding that the
trigger is concurrency"; that sentence counted one variable where the run changed two, and trap 318
records it.

**What stays ruled out, from round 1916 and nothing new**: an unbounded shader loop, and an invalid
command stream as the Khronos validation layer reads one on lavapipe. **What this round checked in
the source and set aside**: a timestamp query resolved before it is written, which on RADV waits for
the query's availability in the command processor and would stall it with no wave to kill — the one
mechanism that matches "no soft recovery". The route that writes only the end timestamp
(`Route::Untouched` in `raster-gpu`'s `device/record.rs`) resolves both slots; but `wgpu-core` 30
resolves a slot never written in any submission as a buffer clear rather than a
`vkCmdCopyQueryPoolResults` (`process_deferred_query_set_resolves`), and a slot written by an earlier
frame holds an available value, so neither case can reach the wait. The clip test's frames are
first frames of fresh devices, with full damage, and every full-damage route stamps both slots.

## 3. What stands

Every guard of ADR 2665 stays: the lavapipe pin, the adapter log, the hang marker that stops every
later hardware section after a hold past nine seconds, and the rule that no hardware suite runs on
this card. `clip_that_cuts_nothing::a_clip_that_contains_a_mark_takes_nothing_from_it`,
`retained_frame::a_zoom_step_misses` and the `sub_pixel_coverage` tests stay withheld as suspected
hangs: the run gave no cause, so no fix, so there is nothing whose cover they could be judged by,
and their one `--exact` run each was conditional on one.

The upstream report of ADR 2665 §4 stays drafted and unfiled: it promises a dump, and there is none.

## 4. The next step, which is the owner's

Q777 asks for two more supervised runs by the same script, each once, in this order, because each
answers one of the two variables of section 2:

1. **The same test alone, without `RADV_DEBUG`.** If it hangs, the hang lives inside the one
   process's own work and the serialising mode hid it; the dump is then out of reach by RADV's own
   mode, and the next instrument is the kernel's (`amdgpu`'s `amdgpu_gpu_recover` devcoredump, or
   `umr`, which this machine does not have).
2. **The shape of 14:02: `clip_that_cuts_nothing`, `retained_frame` and `sub_pixel_coverage` in one
   parallel `nextest`, on RADV, inside the script's outer lock**, each process's hook pointed at one
   shared private lock so that our device sections serialise among themselves exactly as at 14:02.
   If it hangs, concurrency of our own processes is the trigger, and the hang marker stops the rest
   of the run after the first reset. The script would gain the mode of running a command rather
   than one test binary for it.

If neither hangs, what is left is the other user's concurrent work and the card's state after the
morning's resets, neither of which is ours to reproduce, and the guards stay as the answer.

## Cost

One second of the shared card under an exclusive lock, and no reset. A script in `tools/` that only
this kind of run uses; it is kept because Q777's runs are the same procedure and a procedure whose
order matters (lock before test, reset before release) is safer as code than as a paragraph.
