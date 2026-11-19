# 3093 — The GPU lock is taken through the server's FIFO queue

Session 2098. Status: **implemented.** Context: ADRs 2664 (the code holds the lock around device work), 2675 (a
device's whole life is device work), 2665 and 2740 (the hang marker). The server's `/usr/local/bin/with-gpu-lock`,
changed on 2026-11-18.

## The problem

The server's wrapper took a bare `flock` on `/run/lock/gpu.lock`, and a bare `flock` is granted to whichever
waiter wakes first. A process that releases the lock and asks again at once is usually that waiter, so one
agent's suite could take the card back section after section while the other agent's single job waited. The
wrapper now serves waiters in arrival order: each writes a ticket `<arrival_ns>.<pid>.<start_ticks>.<deadline_s>`
in `/run/lock/gpu.queue` (sticky, mode 1777), waits until its ticket is the earliest live one, and only then
takes the `flock`, which stays the actual exclusion so that callers of the old wrapper still interoperate.

`tools/conformance/test-support/gpu_lock.rs` takes the same `flock` itself (ADR 2664), so without a ticket our
sections jumped that queue: they were exactly the bare-`flock` waiter the change was made against.

## The decision

`gpu_lock::enter` joins the queue the way the wrapper does, and nothing else about the lock changes.

- **Where.** The queue is the directory beside the lock file named as the file with the extension `queue`
  (`/run/lock/gpu.queue` beside `/run/lock/gpu.lock`), used only where it is a directory and not a link. The code
  never creates it: a machine whose lock has no queue (or a test's own lock file) keeps the bare `flock`.
- **The ticket** is the wrapper's: arrival in nanoseconds since the epoch, zero-padded to twenty digits so that
  byte order is arrival order; the pid; the process's start time in clock ticks (field 22 of `/proc/<pid>/stat`,
  counted after the command name as the wrapper counts it); and a deadline of arrival plus `GPU_LOCK_TIMEOUT` plus
  two seconds, the wrapper's own slack. It is created with `create_new`.
- **Liveness** is the wrapper's: a ticket waits while its process runs with that start time and its deadline has
  not passed. On each look (every 25 ms) the earliest live ticket is the head; stale tickets owned by this user
  are removed on the way, other users' are left to them, and names of any other shape are ignored. The owner is
  compared with the owner of the ticket this process just wrote, so no system call is needed.
- **Then the `flock`**, under what is left of the same bound: `GPU_LOCK_TIMEOUT` covers the queue's wait and
  the lock's together, as the wrapper's single deadline does. The ticket is removed once the lock is held.
- **Every way out removes the ticket.** The ticket is a guard whose `Drop` removes the file, so a timeout's
  panic, a failed `try_lock` and success all leave the queue as they found it.
- **Unchanged**: the in-process re-entrancy (a second section in a process counts a holder and never queues),
  the hold log, the hang marker. The hold log's line gains a last field, `ticket=<name>` or `ticket=none`,
  so a run's log shows which holds were served through the queue; `tools/gpu-holds.sh` reads fields by name and
  is unaffected.

## Why not call the wrapper

Running `with-gpu-lock` around each section would need a child process holding the lock across the section's
life, and the section's own process would then hold nothing it could release on drop; the hook's span (bring-up
to drop, ADR 2675) is in-process. Matching the wrapper's protocol keeps the span where it is.

## Tests

`crates/render-raster/tests/gpu_lock.rs`, each against a lock file and a queue directory of its own under the
temporary directory, never the machine's: `queued_sections_are_served_in_arrival_order` (two children queued
behind a planted holder hold in their arrival order, each on a ticket, and leave the queue empty);
`a_free_lock_waits_behind_an_earlier_live_ticket` (a free lock is not taken past a live ticket that arrived
first — the one behaviour a bare `flock` cannot show); `a_dead_ticket_is_skipped_and_removed` (a dead pid's
ticket and an expired one hold nobody back and are removed); `a_timeout_in_the_queue_removes_the_ticket`;
`a_ticket_s_name_is_read_as_with_gpu_lock_writes_it` (the name's grammar, and the start time read as the
wrapper's own `bash` reads it).
