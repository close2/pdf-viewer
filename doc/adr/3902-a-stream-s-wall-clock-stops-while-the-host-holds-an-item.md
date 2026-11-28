# 3902 — A stream's wall clock stops while the host holds an item

Session 2300. Status: **accepted and built**. Code: `confined-transport`'s `stream.rs` (`Watch`,
`Watched`, `Budget::wall`); `pdf-media-worker`'s `client/video.rs` (`wall`, comment only). Tests:
`stream::tests::the_wall_clock_stops_while_the_host_holds_an_item`, `pdf-media-worker`'s
`tests/video.rs::a_caller_that_holds_its_pictures_does_not_spend_the_worker_s_wall_clock`.
Amends ADR 3527's deadline as `confined-transport` counts it.

## Context

Under four lanes' load, `viewer-gtk`'s `a_clip_plays_to_its_end_in_the_gtk_window` failed with
"the media worker did not answer within 2.28s", and passed when run alone (rounds 2295, 2297).
The 2.28 s is the product's own bound, not the test's: `client::video::wall` gives a video run
`PROCESSOR_BUDGET`, one second, plus the run's media time, here 1.28 s. `Budget::wall` was the
time from the open request to the worker's `END`. But the window drains the decoding thread by
its presentation clock (`AHEAD` pictures queued, `decode_pass`'s send blocks), and the stream
grants credit only when the host reads again. So the worker spent most of the run waiting for
credit the host had not granted yet, and the bound was spent on the host's pace. A window
running a little late, for any reason a loaded machine gives it, used up the remaining second.
The worker was then killed as stalled and distrusted, though it had stalled on nothing.
Reproduced deterministically without load: a caller holding each of forty 40 ms pictures for
80 ms failed with `Stalled { after: 2.28s }`.

## Decision

The wall clock counts the time the host waits on the worker. From the moment an item is handed
out until the host's next read, the watch is stopped (`Watched::Held`, then `Watched::Reading`).
A worker that stalls while the host is reading is still ended at the same bound
(`stream-witness`'s `a_worker_that_stalls_is_ended_at_the_wall_clock_budget` unchanged). The
test bound was not widened and nothing is retried: the product was counting the wrong time.

## Consequences

- A host slower than real time, such as a paced window, a loaded machine, or a debugger, no
  longer costs a clip its worker.
- Work the worker does while the host holds an item is not counted. The bound is still finite
  against the host's own pace: once the host reads, the remaining budget runs out if the worker
  sends nothing.
