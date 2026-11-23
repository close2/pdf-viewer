# 3468 — The loopback's out-of-order pairs are its copy at a playback xrun

Session 2191. Status: accepted. Context: ADR 3459; review 2185's round-2189 section ("4 462
out-of-order frames … in pairs of a 1 024-frame step back and one forward").

## Reading

A pair reads, for example, `19455 -> 18432` then `18559 -> 19456`: after frame 19 455 the capture
took 128 frames from one buffer earlier, then went on at 19 456 with none lost. The helper's buffer
on the loopback was 1 024 frames (two periods of 512, read from its trace), so "one buffer earlier"
is the step back.

That is `snd-aloop`'s own behaviour, from its source (Linux 6.12, `sound/drivers/aloop.c`):
`copy_play_buf` copies the playback ring buffer to the capture side for the time elapsed, and trims
the copy at the application's pointer only when the playback is `DRAINING`. A running playback
whose application is late therefore has the ring's stale contents — the samples of one buffer
before — copied, and the core declares the xrun after, at the period's update. `cpal` then prepares
the stream and the helper's next samples follow on, which is the step forward. A card's DMA
replays its ring the same way until its interrupt finds the xrun.

So the pairs are the helper's xruns — the helper was late — in the shape this device gives them,
and they are counted as underruns, two lines to one. The helper's remedy is its period (ADR 3466),
not anything about the pairs.
