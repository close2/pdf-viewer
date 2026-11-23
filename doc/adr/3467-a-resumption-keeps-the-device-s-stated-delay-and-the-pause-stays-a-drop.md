# 3467 — A resumption keeps the device's stated delay, and a pause stays a drop

Session 2191. Status: accepted. Context: ADR 3458 (the frame the device says it reached); review
2185's round-2189 section ("on the pipe the device's stated delay is a little short … on the
loopback a little long"); ADR 3466.

## Decision

The resumption stays ADR 3458's: the frames written less the delay `cpal` stated, from the last
callback's instant, with no constant added. `PAUSE` still takes the instant, reads the last
callback's timing and drops the stream; it does not pause the device first.

## What was measured (2026-11-23, three trials a run)

| | a resume skips (+) or repeats (−) | pause → last frame |
|---|---|---|
| loopback, device's period, load 20–144 | +1.5, +2.5 ms | 1–3 ms |
| loopback, 20 ms period, load 127–212 | −1.1, −6.4, −10.4 ms | 11–23 ms |
| loopback, 20 ms period, paused first, load 15–38 | +6.1, +1.0, +0.1 ms | 1–14 ms |
| pipe, 20 ms period | −14.9, −1.1, −9.5 ms | 50–64 ms |
| pipe, 20 ms period, paused first | 0, −20.0, −20.0 ms (two runs) | 97–573 ms |

## Why the device's report is kept

Two errors remain and neither is the report's. Dropping a `cpal` stream stops the device only once
the stream's thread has woken and been joined, so on a loaded machine the device plays on for a
few milliseconds after the instant, which the resumption repeats; and `snd-aloop` advances by the
kernel's tick (`CONFIG_HZ=250`, 4 ms) and states its delay in steps of 1.33 ms (the trace's
delays), so a reading there is good to about a tick either way. On the pipe, ALSA's `null` states
a delay of 0 and the `file` plugin writes the two periods it holds back when the stream closes, so
the pipe's repeat is its own plumbing, which no device report could carry. A constant fitted to
either stand-in would be fitted to a stand-in.

## Why the pause stays a drop

Pausing the device first (`StreamTrait::pause`, on the helper's own thread) took the loopback's
error inside its tick. But on the pipe it made the pause's tail 97–573 ms where a drop's is 50–64,
and a sampler of the helper's threads showed the helper finished within 61 ms of its last callback,
so the rest is outside the helper and not yet explained. A change whose effect on one device is not
understood does not ship; the join's milliseconds are the smaller cost. Reopened when the pipe's
tail under a pause is explained.
