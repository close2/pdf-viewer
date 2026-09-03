# ADR 0807 — A bound that is not being measured is not there: the tree sampler walks once, samples against a deadline, and stops a tree it has gone blind on

Status: accepted. Session 880.
Clauses: none — this is about the instrument every corpus walk runs under.
Code: `tools/bounded.sh` (`walk_table`, `process_table`, `sample_tree`, `watch_tree`, `--self-test`).
Tests: `tools/conformance/tests/bounded.rs::the_bounded_wrappers_self_test_holds`, which runs
`tools/bounded.sh --self-test` under `cargo test -p conformance`.
Documents: the script's own header; `doc/environment.md`'s parallel-round agreements are unchanged.

## Context

`tools/bounded.sh` is the wrapper the owner's four rules of 2026-09-02 put every walk, census
and build under (ADR 0798, `doc/environment.md`). Its `--tree` ceiling is the only bound on this
machine that sees a *sum* — a `cargo build`'s memory is spread over `rustc` processes no
`RLIMIT_DATA` sees, and a survey's over a rayon pool and the sandbox workers it spawns — and it
is enforced by a sampler: once a second, `ps -eo pid=,ppid=,rss=`, an `awk` that walks the tree
under the wrapper's leader and sums the resident sizes.

Round 874 watched that sampler hang for minutes on a survey wrapper and killed it by pid. Read
against the script, three things were wrong with it, and none of them was the `ps`:

- **The walk was quadratic.** For every node reached, the `awk` scanned *every* row of the table
  looking for children — `for (c in parent) if (parent[c] == p)` — so a sample cost the table's
  size times the tree's. Measured on a synthetic table: 0.07 s over a flat tree of a thousand,
  0.66 s over four thousand, 6.4 s over eight thousand, 16 s over sixteen thousand.
- **A pid could be visited twice.** Nothing marked a node seen, so a cycle in the table — which
  `ps` can assemble from pids reused while it reads `/proc`, and which a duplicated row is — was a
  queue that never emptied.
- **A sample had no deadline.** The loop was `tree_rss; sleep 1`, so a `ps` that stalled — which
  is what a `ps` does on a machine whose memory is going, the exact condition the ceiling exists
  for — stalled the bound with it. The wrapper was blind precisely when it was needed, and it did
  not say so.

The first two are what the round was asked to find and the third is what makes them matter: a
sampler that can take minutes, or forever, is a bound that is not being enforced, and the
2026-09-02 incident is what an unenforced bound costs (`tools/bounded.sh`'s header has the
timeline).

## Decision

1. **One `ps` a sample, walked once.** Children are gathered per parent in a single pass —
   indexed (`kid[parent, n]`), not concatenated, because a string of a hundred thousand pids grown
   one at a time was a second and a half by itself — and the tree is walked from the root through
   those lists, each pid marked seen. The self-test's hundred-thousand-row table samples in about
   a hundred milliseconds where the old walk needed minutes; a chain fifty thousand deep, a cycle
   and a duplicated row walk once each.

2. **A sample runs against a deadline.** `sample_tree` runs the `ps` and the walk in the
   background and waits at most five seconds for it. A sample that misses is abandoned — disowned
   and sent `SIGKILL`, which a process blocked inside the kernel will not act on until it returns,
   so it is never *waited* for — and the wrapper says on standard error that it missed, and which
   miss of how many this is.

3. **Six misses in a row stop the tree.** Half a minute blind is the figure: on 2026-09-02 every
   shell call on the machine stalled for three minutes before the session was lost, and a wrapper
   that had acted within thirty seconds would have ended the walk that caused it. The kill list
   is the last good sample's pids plus the leader — no fresh list can be had, that is the
   condition — and the last line names the cause, `KILLED BLIND`, with how long the ceiling went
   unmeasured and what else to look at, distinct from `KILLED BY THE TREE CEILING`. Exit 137
   either way.

4. **The sampler is tested where the sequence will run it.** `--self-test` exercises five cases
   in the script's own functions — the synthetic tables, a live tree that fans out into two
   hundred children, a child that holds 1.5 GiB under a 1 GiB ceiling and is stopped with the
   ceiling's line, and a sampler replaced by one that never returns, stopped within seconds by the
   blind rule — and `tools/conformance/tests/bounded.rs` runs it, so `cargo test -p conformance`,
   the sequence's last line, fails a round whose wrapper is broken before that round walks
   anything under it. The `ps` behind the sampler is a function so that the last case can stand a
   stalling one in its place without a knob a caller could reach.

## Consequences

- One sample costs milliseconds whatever the tree's size, and cannot outlive its interval by more
  than the deadline. A wrapper that cannot measure stops the walk rather than running it
  unbounded, and says which of the two bounds ended the run.
- The blind rule can stop a walk the machine would have survived: a `ps` slower than five seconds
  six times running is the condition, and nothing else on this machine produces it. If it fires
  on a quiet machine, the sampler is the thing to look at, and the line says so.
- The self-test allocates 1.5 GiB for a few seconds under `cargo test -p conformance`, and wants
  `python3` to do it; where there is none the case prints `NOT RUN` and the Rust test fails on
  that line rather than passing without it, because CI has one.
- What was *not* done: replacing `ps` with a read of `/proc/[0-9]*/stat` from `awk` directly, or a
  cgroup. The first saves a fork and changes nothing about the three faults; the second is the
  right instrument and is the owner's to set (ADR 0798), because `systemd-run --user` is not
  available to this account.
