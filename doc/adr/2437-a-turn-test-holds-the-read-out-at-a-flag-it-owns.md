# 2437 — The turn test holds the read out at a flag it owns

Session 1801, a "host" round in batch 120, 2026-10-27.
Status: implemented in `crates/viewer-ffi/src/window.rs` (the test-only `holding` field) and
`crates/viewer-ffi/src/window/turn_tests.rs`. Context: ADR 2344 (a turn's read leaves the window's
thread), ADR 1780 (a test that needs work in progress holds it open), CI run 40431170532.

## The red

CI's `test` job on b46ff72e failed `window::turn_tests::a_turn_returns_with_its_read_out_and_a_second_turn_waits_for_it`
at "a second turn waits for the first's read", left 0, right 1. The test passed on 88339689's run
and passes here. The test dispatched a turn, then a second one, and assumed the first read was
still out when the second key came. The page is a single fill. Its read runs on a thread of its
own, and every `drain` ends by looking for that read's end (`take_the_drawn`, then
`take_the_turn`). If the read ended before the second key's drain looked, the window took it in.
It then handed the second turn on at once, as the production path should, and nothing was waiting.
Nothing in the test held the read out.

## Decision

**A test-only `holding` flag on `Window`.** While it is set, `drain` and `tick` do not call
`take_the_drawn`. The test sets it before the first key and clears it before `rest`.
The read is held out by something the test owns, which is the one condition the test is about. The
read may end at any time on its own thread. The window takes nothing in until the test lets it, so
`state.turning` stays `Some`, and the second key meets a read that is still out. The check the
production path makes is `through_the_viewer`'s `turning.is_some()`, and it is unchanged.

**Why at the window's look and not inside the read.** The read is `viewer_core::ReadOut::read`,
run on a thread `viewer_host::Ahead::read_out` spawns. Neither crate has a hook a test could hold,
and both are outside this fix's files. The document's bytes are a regular file read with `pread`,
which nothing can hold. The look is the narrowest point in `crates/viewer-ffi/src/window/` that
decides "out" against "landed". The non-test build compiles the same statement it always did
(`#[cfg(not(test))]`), so the production path is the same byte for byte.

## Evidence

- The old test, 200 runs alone at a load average of 37 to 39: 0 failures. Pinned to one core: 0.
  The whole library test binary on four cores (CI's runner has four): 0 in 200. The race is CI's
  and does not reproduce here.
- **Calibrated by forcing the race** (trap 13): a 300 ms pause planted between the two keys makes
  the old test fail every time and leaves the new test passing (see the record). Both plants were
  reversed by their own patches.
- The new test, 200 runs at a load average of 33 to 37 with a build of this crate running beside
  it: 0 failures.
