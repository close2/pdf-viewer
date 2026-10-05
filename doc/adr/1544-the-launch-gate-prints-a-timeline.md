# 1544 — The launch gate prints a timeline, one milestone a line, from the process's spawn

Status: accepted. Session 1354. Amends ADR 1531 section 1 in one respect: the first-page phase's
`device_ms`, `joined_ms` and `interpreted_ms` line becomes a timeline of eight milestones, and
`interpreted_ms` is renamed `requested_ms` for what it always measured. Supersedes nothing.
Context: `CLAUDE.md` principle 2; `doc/habits/measuring.md` habit 66 ("a launch gate that prints
'device up' and 'document joined' at one instant hides the work done after the join"); ADRs 0884,
1531, 1543.
Code: `crates/viewer-ui/tests/launch_path.rs` (`SPAWNED`, `spawned_before`, `print_timeline`, the
first-page phase's two threads and its preparation thread, `run_phase`).

## 1. What the line hid

The first-page line read "device up at 22.8, document joined at 22.8, page one interpreted at
22.8" on four rows of five. Both instants are the main thread's: the device returned, and the join
returned at once because the document thread had long finished. What that thread did, and how long
before the join it was idle, was not on any line, so which thread a launch waited for took
callgrind to say — and on `bug1815476.pdf`, the one row where it is the document, the three
figures still sat within half a millisecond of each other.

## 2. The shape

Each figure the child already took is kept and three are added: when the document thread's open
returned (`opened_ms`), when its anticipation of page one returned (`anticipated_ms`), and when the
outline and page tree ADR 1543 moves off the launch path were read on their own thread
(`prepared_ms`, joined after the frame's clock has stopped, so it is in no figure). The parent
passes the instant it spawned the child (`PDFVIEWER_LAUNCH_SPAWNED_NS`, the system clock on both
sides because an `Instant` does not cross a process), so the timeline starts where a launch does.
The run prints one line per milestone, the two threads interleaved in the order they happened:

```text
launch-path:         0.0 ms  process spawned
launch-path:         1.5 ms  the phase begins — process, exec and the test harness before it
launch-path:         6.6 ms  document opened (document thread)
launch-path:        11.0 ms  page one interpreted (document thread)
launch-path:        21.1 ms  graphics device up
launch-path:        21.1 ms  document joined
launch-path:        21.2 ms  first render requested
launch-path:        31.6 ms  first frame drawn — read back headless; no window, no present
launch-path:        33.0 ms  outline and page tree read (a thread of their own, after the frame's clock)
```

(ISO 32000-2, one run of ADR 1543's arm.) **The band is unchanged and still judged from the
phase's start**: the spawn offset contains `taskset`'s exec and the test harness, which are this
gate's rather than `quorra`'s, so it is printed and judged nowhere. The headless gate has no
window and no present; the line says "drawn — read back" rather than "presented" so that nobody
reads it as the window's figure, which is still `quorra --trace`'s.

## 3. What it made legible at once

On four rows the device is the launch: the document thread is idle 7 to 19 ms before the join. On
`bug1815476.pdf` page one's interpretation ends 2.2 to 3.0 ms after the device, which is the one row whose
next lever is the document's (ADR 1543 section 5).
