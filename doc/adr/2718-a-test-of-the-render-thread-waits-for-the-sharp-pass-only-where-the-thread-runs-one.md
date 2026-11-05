# 2718 — A test of the render thread waits for the sharp pass only where the thread runs one

Status: accepted and built. Session 1952. Context: ADRs 0699, 0761, 1289; trap 333;
`crates/viewer-device/src/renderer.rs` (`Done::built`), `renderer/thread.rs`, `renderer/gesture.rs`.
The standard says nothing about a render thread's order, so no ledger row is touched.

## What failed

`renderer::gesture::a_zoom_step_is_drawn_before_the_sharp_pass_of_the_view_it_replaced` timed out on
GitHub's runner (CI runs 41908926756 and 42076830573, lavapipe, `cargo test --workspace`). The panic's
line was the `expect` in the test's `next` closure, which all three of its waits call.

## Cause

Not a hang and not slowness: the test's protocol. ADR 0761's rule declines the sharp pass, silently,
where four times the last built frame passes `SHARP_STALL_BUDGET` (400 ms), so a frame built in more
than 100 ms is never sharpened. The test waited for a `Finished::Sharp` after the zoom step whatever
the step had cost. Four copies of the `viewer-device` lib suite at once, pinned to processors 0 to 3
on lavapipe, failed 4 of 4 before the change; `gdb` on one showed the page renderer in `jobs.recv()`
(thread.rs, the loop's last line: nothing left to do) and the test in its third wait. After the
change, 120 runs of the same load printed the step's build at 61 to 241 ms (median 127 ms): 99 had
their pass declined, 21 had it drawn. Alone, the step builds under 100 ms, which is why CI passed most
times and a rerun passed.

## Decision

- **`Done::built`** is the one statement of what a frame says about the device's build cost — `None`
  for a processor's frame (trap 143) or a replay (ADR 0384). The thread's `last_built` reads it, and
  the test reads the same function, so the test's expectation cannot drift from the thread's rule.
- **The test asks `sharp_pass_affordable` of the frame the thread predicts from**: affordable, the
  sharp pass of the new view must come (ADR 1289's assertion, unchanged); declined, nothing may come
  within twice the budget, and a picture that does is a failure. The order is still the only thing
  asserted; the absence wait is bounded by the rule's own constant.
- **Each wait names what it waited for**, so a timeout says which answer did not come.
- The timeout was not raised: two minutes was never the problem.

## Cost

One method on `Done` and a two-branch tail in the test. A run whose step is declined does not check
that the pass is of the new view; it checks that no pass is drawn, which is the thread's rule there.
