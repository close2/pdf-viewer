# 2740 — A section past the hang bound is marked hung only where the kernel recorded the card hanging

Status: accepted; built in `tools/conformance/test-support/gpu_lock.rs` and tested in
`render-raster --test gpu_lock`. Session 1963, a single round. Context: ADR 2665 (the resets are read
from the kernel, and a hang stops every later device test), ADR 2664 (the lock around each device
section, and the clock gates' hold), ADR 2675 (a device holds the lock for its life). It cites no
clause and moves no ledger row.

## What happened

ADR 2665 §3 wrote `<lock>.pdf-viewer-hung` for any hardware section held past `HUNG_AFTER` (9 s), on
the clock alone. The marker stops every hardware section in every lane until a person removes it.
It has been written twice:

| marker | section | `journalctl -k` in the section's window |
|---|---|---|
| 2026-11-05 08:51 | `frame`, held 12 566 ms | 08:51:22.83 `amdgpu: ring gfx timeout, but soft recovered` |
| 2026-11-06 07:40 | `turn-round`, held 9 407 ms | nothing from amdgpu all morning; load average 28 to 31 |

The second section is a clock gate's hold (`frame_cost::round`): three device bring-ups, a dozen
frames and their readbacks under one hold, on a machine whose load reached 677 that day. Nothing in
it was near a ring timeout. It was slow, not hung.

## The decision: (c), the kernel corroborates and the clock only asks

ADR 2665's own argument is that the kernel is the record. Its bound is the kernel's figure read
backwards: "a job the kernel times out holds its section at least that long". That makes the bound
a **necessary** condition of a hang. It is not a sufficient one. A section of many jobs, or a
starved one, passes it with every job fast. So:

- **A section past the bound asks the kernel's log** (`journalctl -k -o short-unix` from one second
  before the section began, waiting up to five seconds for the journal to store the line). It looks
  for amdgpu's ring timeout, a GPU reset, a fault, or an illegal register access. These are ADR
  2665's table of what the five resets printed. **Any process's line counts.** The soft-recovered
  line names no process, and a recovery caused by the other user's job leaves the same card
  recovering under the next suite.
- **A line found → the marker**, which carries the kernel's line beside the section.
- **No line → a warning**: one line appended to `<lock>.pdf-viewer-slow` and to standard error, and
  nothing is stopped.
- **The log unread → the marker**, on the bound alone, saying `kernel=unread`. This covers no
  `journalctl`, a non-zero exit, and a user outside `systemd-journal` (who is answered with no kernel
  line at all, which is how an unread log is told from a quiet one). Here the old behaviour is the
  safe one.

Against the two events: the 08:51 frame's marker was written in second 1793868682 (round 1951's
`turn.log`), and the soft-recovered line is stamped 1793868682.83. The line can therefore come
after the section ends, because amdgpu prints it once the wave is killed and the fence signals.
The five-second wait is what finds it, and the frame is marked as before. The 07:40 turn round's window holds no amdgpu line, so it warns.

**(a) alone**, the kernel without the bound, was rejected. It would need a `journalctl` call after
every section, thousands per suite. The bound is what makes the question cheap to ask.

**(b)**, the bound scaled by starvation, was rejected. A hung job and a starved process both show
little CPU time against their wall time, because waiting on a fence is not CPU work. The load can
be high during a real hang, as it was during 2026-11-03's suites. Neither measure separates the two
cases. The kernel's log does.

**Readability.** The test process runs as `quorra`, which is in `systemd-journal`, so `journalctl -k`
reads the kernel's lines. `/dev/kmsg` is closed (`dmesg_restrict = 1`), and the code does not use it.
CI has no lock file, so it never reaches this code.

## Cost

- A hang the kernel does not log stops nothing now. amdgpu logs every job timeout before it recovers
  or resets, so such a hang would be a section that never ends. The old bound could not mark that
  either: the marker is written when the section ends.
- A section past the bound spends up to five seconds more when the kernel is quiet.
- The window is wall time. A fault from the other user's job inside it marks the card hung for us
  too. That is wanted, because the card is recovering whoever caused it.

## Tests

`render-raster --test gpu_lock` stubs the kernel's log through `PDF_VIEWER_GPU_KERNEL_LOG`:

- the soft-recovered line written while the holder holds the lock → the marker, the next section
  refused, then allowed once the marker is removed;
- an hour-old timeout and a non-amdgpu line → no marker, a warning, and the next section runs;
- no log file → the marker, `kernel=unread`;
- `hang_line` against ADR 2665's real lines, the window's edge, and an NVMe timeout.
