# 1554 — The confined worker rasterises on every processor

Status: accepted. Session 1359. Amends ADR 0218 section 2 (one rasterising thread) and ADR 0241
section 6; supersedes nothing.
Context: `CLAUDE.md` principles 2 and 3; `viewer-core` rule 4, which binds the core and not its
host (`doc/todo/49` item 1, "threads the core was not handed"); ADR 0139 (the strip split draws the
same bytes on any number of strips); `doc/todo/34` item 4.
Code: `crates/viewer-confined/src/worker.rs` (`ARENA_LIMIT_VARIABLE`, `RASTERISING_STACK`,
`confine`, `message_budget`); `crates/confined-transport/src/host.rs` (unchanged: it sets
`MALLOC_ARENA_MAX=1`).
Tests: `tests/confined.rs`'s `a_pool_built_inside_the_confinement_under_an_arena_limit_draws_on_every_thread`;
`worker.rs`'s `a_message_budget_leaves_room_for_two_copies_a_page_and_the_pools_stacks`.

## What held the worker at one thread, and what had already answered it

ADR 0218: `glibc` sizes its arena count from `__get_nprocs`, an `openat` the filter kills for, at
a thread's first allocation. `doc/todo/34` named two ways out — a pool warmed before the filter
(which loses the Landlock domain) and an allocator that does not ask (every candidate has
`unsafe`). A third was already in the tree: `confined_transport::Host::start` sets
`MALLOC_ARENA_MAX=1` for every worker, put there when a `pdf-vfs-worker` died the same way, and
with it `glibc` takes the limit from the variable and never asks.

## Decision

`confine` builds the pool after the confinement, so every thread inherits the Landlock domain and
the filter, as wide as `available_parallelism` answered before it — where `MALLOC_ARENA_MAX` is a
positive number. Where it is not (a process confined some other way, such as a test confining
itself), the pool is one thread, which cannot reach the question. The stack is stated, 2 MiB, and
`threads × stack` is a term of the message budget, since the pool is built after the address space
was read. As wide as the machine rather than a smaller number because the host is idle while the
worker draws page one, and because it is what the unconfined viewer's own pool is.

## Measured

`quorra-confined --trace`, Xvfb, release, one binary with the width chosen at spawn, fifteen
launches an arm, interleaved, 2026-10-06, load average 8: `bug1815476.pdf` — device up to first
frame 0.054 → 0.026 s at the median (0.043–0.061 → 0.021–0.032 s), the first `Resize`'s round trip
0.039 → 0.011 s, first frame 0.146 → 0.122 s. A ten-launch arm over 1, 2, 4, 8 and 24 threads put
4 and more within the noise of each other on that page; WTPDF and ISO 32000-2, whose pages cross
as marks, moved by nothing beyond it. Under callgrind on one thread the page's rasterisation is
143.8 M instructions and its interpretation 121.8 M; the interpretation is before the device is up
and is `pdf-model`'s (round 1361's unpack).
