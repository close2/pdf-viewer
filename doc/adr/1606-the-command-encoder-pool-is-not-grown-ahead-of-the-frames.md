# 1606 — The command-encoder pool is not grown ahead of the frames: measured, and declined

Status: accepted. Session 1385. Answers the lever ADR 1594 section 4 priced and left ("growing the
pool on the warm-up thread would take it off the first frame") and corrects that section's
attribution of the cost, which ADR 1595 section 3 repeats. Supersedes nothing; nothing is built.
Context: `CLAUDE.md` principle 2 ("[n]othing on the launch path waits for warmth"; a lever is
"decided by measurement, never by assumption"); ADRs 1558, 1594, 1595; traps 50 and 116.
Code: none kept. The arm measured was `raster-gpu`'s warm-up thread making `ENCODERS_GROWN`
command encoders, opening each with a debug marker, finishing it and dropping it unsubmitted, so
that it went back to `wgpu-core`'s pool with its command buffers allocated; it is not in the tree.

## 1. What the cost is, counted by function

`wgpu-core` 30 records every render pass into a HAL command buffer of its own and inserts a second
before it for the pass's barriers (`InnerCommandEncoder::open_pass`, `close_and_swap`), so **a pass
costs two command buffers**. A HAL encoder on Vulkan allocates sixteen at once when its free list is
empty (`ALLOCATION_GRANULARITY`), keeps every one it has ever had, and goes back to the device's pool
when its submission is retired; the queue's staging writes take an encoder from the same pool after
every submission. Callgrind, the call counts of `vkAllocateCommandBuffers`, `zoom_frame` on HEAD:

| run | render passes | allocations of sixteen | their instructions |
|---|---|---|---|
| first frame, each of the launch gate's five rows | 2 to 7 | **2** (the frame's encoder and the readback's) | 1.23 to 1.24 M |
| first frame, `personwithdog.pdf` | 28 | 5 | — |
| first frame, `bug1721218_reduced.pdf` | 221 | 17 | 10.5 M |
| eight frames on one device, `PDF20_AN001-BPC.pdf` | 56 | 4 | 2.4 M |
| eight frames on one device, `bug1721218_reduced.pdf` | 1 768 | 84 = six encoders × 14 | 51.5 M |

So ADR 1594's "sixteen Vulkan command buffers at once" is one allocation of fourteen on that page:
a group render records about 110 passes, so an encoder's first use there needs about 220 command
buffers. The 8.6 M of its first `finish` and ADR 1595's "about 3 ms" an encoder are that page's, and
**they scale with the passes, not with the pool**. A launch row's first frame pays two allocations.

## 2. Measured: four arms, one binary, the knobs removed since

The launch gate's `first-page` child, thirty children an arm interleaved child by child, pinned,
2026-10-07, load 1.5 to 1.7; the first frame's `device` stage, min / median, ms:

| arm | `PDF20_AN001-BPC.pdf` | `bug1815476.pdf` |
|---|---|---|
| none (HEAD) | 5.47 / 6.19 | 7.10 / 8.08 |
| six encoders after the warm set | 5.47 / 6.66 | 7.89 / 9.23 |
| two encoders before the warm set | 5.82 / 6.20 | 6.89 / 8.00 |
| three encoders on a thread of their own | 5.63 / 5.90 | 6.70 / 7.77 |

After the warm set — the arm the lever was priced as — the growth runs 4.8 to 6.1 ms after the
device is up, which is during the first frame, and the frame is slower by 0.5 to 1.2 ms at the
median. Before it, the warm set starts later and the frame gains nothing. On a thread of its own the
pool was ready 0.3 to 0.7 ms after the device and the frame read 0.3 ms quicker at the median and
no quicker at the minimum, inside the spread. `zoom_frame`'s eight-frame sequence on one device,
three runs of five rounds an arm interleaved (load 2.8 to 3.3): `bug1721218_reduced.pdf`'s seventh
frame 69.6 to 71.4 ms on HEAD, 70.4 to 70.8 after the warm set, 69.8 to 72.0 before it; frames 1 to
6 and `personwithdog.pdf`'s eight inside each other's spread. Six encoders cost the warm-up thread
1.3 to 1.7 ms and the process 2.2 MB of resident memory; growing each to the 224 command buffers
the heavy page needs (112 empty compute passes an encoder) cost 37 MB.

## 3. Decided: not built

A growth that runs beside the first frame costs that frame more than the two allocations it would
spare it, and on the launch path the first frame begins the moment the device is up, so there is no
idle moment on that thread to grow into. The page that does pay — 14 allocations an encoder — would
need the depth as well as the count, which is 37 MB of command buffers on every device whatever it
draws. Neither is a lever this tree should take; principle 2's rule decides it, and a thread of
its own is not built for a figure inside its spread.

## 4. What is left

The heavy page's cost is its render passes: two command buffers each, about 220 a group render, and
a long-lived device that has drawn it keeps every one. Fewer passes a group render is the lever, and
it is raster's composition (ADRs 1471, 1529), not the pool. The long-lived device's own figure is
now `turn_path`'s `seventh` row (ADR 1607), so a change of that kind is judged on the device a window
keeps as well as on a fresh one.
