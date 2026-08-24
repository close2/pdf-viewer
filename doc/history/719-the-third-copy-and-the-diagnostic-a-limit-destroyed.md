# 719 — The third copy, and the diagnostic the confinement destroyed

`doc/todo/15`'s road B, taken: a confined worker held three copies of every document it opened, and
`RLIMIT_FSIZE` was killing it before it could say so. ADR 0597. Date 2026-08-25.

## What the round was sent to settle first, and what the answer changed

Road D closed in 714, so the briefing asked whether the four-gibibyte ceiling is still *for*
anything. It is not for the bomb: Bomb A and Bomb B, rebuilt from `doc/todo/10` §2's description
for the fourth time and coming out at 389 317 and 1 847 467 bytes to the byte, move the confined
worker's `VmPeak` by **nothing** — it sits at 147 568 KB, 3.5% of the ceiling, before and after
each, the same figure an ordinary five-page document leaves it at. The peak is reached during
start-up.

**Asking what still reaches the ceiling is what found the defect**, and it is the ordinary thing
rather than the hostile one. A valid document padded to a stated size, opened in the confined
worker:

| document | `VmPeak` before | after |
|---|---|---|
| 999 996 130 B | 3 011 720 KB | 2 035 164 KB |
| 1 399 996 132 B | 4 183 592 KB — **99.7% of the ceiling** | 2 816 412 KB |
| 1 499 996 132 B | the worker died | 3 011 728 KB |
| 1 599 996 132 B | the worker died | **refused by name**, `VmPeak` unmoved |

The left column is exact arithmetic: **start-up size plus three times the document's length, to the
kilobyte, at every size measured.** The frame buffer, the copy `decode_command` makes, and the
`Arc<[u8]>` `pdf_syntax` makes. The first of the three was free — everything a decoded message owns
is already its own, so the buffer is dead the moment the decode returns — and dropping it there is
the right-hand column, again exact.

## The finding the round was not sent for

The 1.9 GB document killed the worker. **What the host was told depended on where its own standard
error pointed**, and neither answer was the truth:

| the host's standard error is | the host is told | the worker said |
|---|---|---|
| a pipe | `killed by signal 6` | `memory allocation of 1899996152 bytes failed`, to a terminal |
| a file | `killed by signal 25` | nothing at all |

Signal 25 is `SIGXFSZ`. `RLIMIT_FSIZE` is 0 in the confinement and the worker's standard error was
the *host's*, inherited — so on every logged deployment the worker's own explanation is a write
that exceeds a file-size limit of zero, and the diagnosis arrives as a file-size failure for an
out-of-memory abort. `doc/todo/15` had this recorded as "indistinguishable from a crash"; it was
misattributed as well. It is trap 18 now, because the shape generalises to any limit and any
report channel.

## What was built

A message budget the worker derives from its own ceiling — `(ceiling − VmSize at the baseline −
a settling allowance − a page's pixels) / 2`, every term read or measured, and the last copy is an
`Arc<[u8]>` that cannot be a `try_reserve` on stable, which is what makes it a bound rather than a
fallible allocation. A document past it is refused *before the first byte is read*, in a sentence
naming the size, the ceiling, the factor and the budget, and **the worker survives with whatever it
had open** — which is why the "worker restart plus document re-open" the item asked for is not owed
for that population. `try_reserve` where a length the other side stated becomes an allocation, on
both sides of the pipe, the host's frame payload having had no guard at all. And the worker's
standard error is a pipe, drained on a thread, echoed onward and carried into `WorkerDied`.

## The tier change

Priced, not made. Page one through a pipe on a 173 kB document is about 1.1 ms of spawn and
confinement, 1.8 ms of document and 4 ms of raster on top of a 5 ms page — roughly double, and the
*raster* dominates. On ISO 32000-2 it is 65–75 ms against 26–48 ms. Load average 13–27 from three
parallel rounds, both arms in one sitting, ranges rather than points. `doc/todo/34` §2's question —
display lists across the boundary, or `wgpu` inside the confinement — is still unargued and is what
road B is now waiting on.

## The sequence

Whole, and it was owed whole: a sandbox change can move any gate. Both workers built first.

## Tests

`a_frame_over_the_budget_is_read_past_rather_than_allocated` (two frames back to back, the
discriminating one) · `a_frame_inside_the_budget_is_read_whole` · four on `message_budget`'s
arithmetic · `a_confined_worker_cannot_write_a_diagnostic_to_a_file` (the `SIGXFSZ` finding, pinned
on one write) · `a_worker_that_dies_saying_something_says_it_to_the_host`, run first against the
inherited descriptor, where the host prints exactly *the confined viewer stopped without answering
(exited with status 3)*.

## Ledger

Untouched. This is `CLAUDE.md` principle 3 and cites no clause.
