# 1952 — A test waits for the sharp pass only where the render thread runs one

Session 1952, a single round on a recurring CI failure, the ledger lane. ADRs 2718 and 2719, trap 333;
no question, no ledger row (a render thread's order is nothing the standard states).

**The premise did not hold.** `a_zoom_step_is_drawn_before_the_sharp_pass_of_the_view_it_replaced`
(CI runs 41908926756, 42076830573) panicked at line 167, which is the `expect` inside the test's
`next` closure that all three waits call — not the first frame. Neither a hang nor slowness: the
test waited unconditionally for a sharp pass that ADR 0761's rule declines, silently, where the
frame it doubles was built in more than 100 ms.

**Evidence.** Alone on lavapipe, 30 of 30 passed; the whole lib suite on 4 and on 2 processors,
30 of 30 and 20 of 20. Four copies of the suite at once on processors 0–3: 4 of 4 timed out. `gdb`
on one: the page renderer idle in `jobs.recv()`, the test in its third wait (gesture.rs:196, the
sharp pass). After the fix the test prints the build costs: under that load the step built in
61–241 ms (median 127, n 120), 99 passes declined and 21 drawn; alone the first frame built in
202–209 ms and the step under 100 ms.

**Fix (ADR 2718).** `Done::built` states what a frame says about the device's build cost (`None` for a
processor's frame or a replay); the thread's `last_built` and the test both read it. The test asks
`sharp_pass_affordable` of the frame the thread predicts from: affordable, the pass of the new view
must come; declined, nothing may come within twice `SHARP_STALL_BUDGET`. Each wait names what it
waited for. The two-minute patience is unchanged.

**Finding (ADR 2719).** On lavapipe the first frame is built in over 100 ms, so the first view's pass
is always declined and the race ADR 1289 is about is never exercised on CI; the test says so on its
output. On the RX 580 the first frame built in 89.7 ms and the step in 25.7 ms, so it is exercised
there. What the first frame's 200 ms on lavapipe is made of is owed.

**Repeat runs after the fix**, the lib test binary run directly under `WGPU_BACKEND=vulkan
VK_ICD_FILENAMES=…/lvp_icd.json`: the test alone (`--exact`) 30 of 30; the whole suite under
`taskset -c 0-1`, 30 of 30; four copies at once under `taskset -c 0-3`, 30 rounds, 120 of 120.

**RX 580** (`tools/hang-capture.sh --command`, nextest of the archived `viewer-device` suite,
`RADV_DEBUG=hang`): 95 of 95 passed, 1 skipped, lock held 7 s, kernel log clean, `vulkaninfo` exit
0. The scripted run ADR 2675 owed for `viewer-device` is done; `render-gpu`'s is still owed.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` (`viewer-device`): exit 0. nextest
`viewer-device` on lavapipe: 95 of 95, 1 skipped. `cargo test -p conformance` after `git add -N`
and `state.sh --round 1952 docs`: exit 0.
