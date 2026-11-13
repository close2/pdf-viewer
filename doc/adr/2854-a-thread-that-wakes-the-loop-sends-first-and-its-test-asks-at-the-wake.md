# 2854 — A thread that wakes the loop sends first, and its test asks at the wake

Session 2018, a single round on a race `viewer-ui`'s own test found. Status: **accepted, built**.
Context: ADR 1936 (the `--cpu` composing thread wakes the loop), ADR 1926 (the render thread's wake),
ADR 1831 (the looks). It cites no clause and moves no ledger row.

## 1. The question

In round 2011's merge gates, under load, `a_finished_frame_wakes_the_loop_after_it_is_sent` failed
once: "the wake came before the frame it is for was sent". Either the composing thread can wake the
window's loop before the frame is in the channel — a loop that wakes, finds nothing and rests then
misses the frame until the next wake — or only the test's observation was unordered.

## 2. What was found

**The thread's order is right; the test's observation was not.** `compose_until_told_to_stop` sends
`Finished::Frame` and then wakes, and `compose`'s look sends `Finished::Partial` and wakes only if
that send succeeded. But the test took *the first wake* to be the frame's. A draw that runs past
`viewer_host::drawing::GLIMPSE_AFTER` (500 ms) is looked at, and the look is sent and woken for
before the frame; an empty 60 × 84 page does that on a starved machine. The test then counted no
frame in the channel and failed, though every wake had come after its own news.

Measured with the test alone in a loop, the test process at `nice 19` under 64 busy processes at
`nice 10`: the old test failed 2 of 4 400 runs (0 of 2 020 unloaded). A diagnostic copy of the new
test printing what each wake found saw a look before the frame in 9 of 4 000 runs, and never an
empty wake.

## 3. What is built

- **The test asks on the composing thread, at the moment of each wake.** The wake closure drains the
  channel before it returns and hands what it found to the test; every wake must find at least one
  message, and the test reads wakes until one finds the frame. A wake sent before its news finds the
  channel empty whatever the scheduler does, so the test is deterministic in both directions: the
  new test failed 0 of 6 400 runs (4 400 under the same load), and with the frame's send and wake
  swapped it failed 200 of 200.
- **The rule is written where it binds**: the comment over the frame's wake says the send comes
  first and why.

## 4. The same pattern elsewhere

Every other thread that wakes `quorra`'s loop was read for wake-before-publish: `viewer-device`'s
render thread (frame, sharp pass, scene, read, partial) and `read_back`'s `send`;
`viewer-host`'s page-turn read, submission, URL and policy fetches and archiving; the machine-faces
search; the accessibility bridge. Each publishes (a channel send, or a flag stored `Release` and read
`Acquire`) before it wakes. One weaker form is recorded, not changed: the page-turn read's
`Ends::drop` stores its flag and wakes before the thread's closure has returned, so the woken loop's
`join` may block for that moment; it never misses the result.
