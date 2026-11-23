# 3466 — The helper asks the device for a 20 ms period, and for no scheduling priority

Session 2191. Status: accepted. Context: ADRs 3317, 3459, 3465, 3468; review 2185's round-2189
section ("the helper's scheduling priority and its buffer"); RFC 0015 section 9.1.

## Decision

1. **Period.** The helper asks `cpal` for `BufferSize::Fixed` of 20 ms in the device's frames (960
   at 48 kHz), where the device's stated period range holds it, and otherwise for the device's own
   choice. `cpal` makes a fixed period's buffer two periods, so the buffer is 40 ms.
2. **Priority.** The helper asks for none, and `cpal`'s `realtime` feature stays off.

## What `cpal` lets the helper ask, on ALSA

Only the period: `cpal` 0.18.2's `BufferSize::Fixed(x)` sets the period near `x` and the buffer
near `2x`, with a start threshold of two periods and an `avail_min` of one; `Default` constrains
neither and takes the device's choice. The callback is woken when a period is free, so the time the
helper's thread has to be scheduled before the device runs dry is what the device still holds then.

## The measurements (loopback, a 60 s sound played four times, 2026-11-23)

| | device's choice: period 512, buffer 1 024 | 20 ms: period 960, buffer 1 920 | 20 ms, a later run |
|---|---|---|---|
| load average over the long sound | 20–144 | 127–212 | 15–38 |
| frames held when a callback began (stated delay), median | 5.3 ms | 16 ms | — |
| callbacks later than the frames the previous one left (trace) | 3 814 of 38 033 | 102 of 20 280 | — |
| silences in the long sound | 291, 1.49 s | 0 | 0 |
| frames out of order over the run (passes and pauses included) | 1 711 | 124 | 21 |
| activation → first frame, cold / warm, median | 75 / 27 ms | 393 / 82 ms | 79 / 47 ms |

The device's own choice left the helper 4–7 ms of samples at a wakeup, which a loaded machine's
scheduler overran about once in ten callbacks; twenty milliseconds' period left 12–16 ms and took
the long sound's silences to none at a higher load. The price is start latency, up to a period:
the warm start's median rose by 20 ms at comparable load. On the pipe the period changes nothing
the reader sees — ALSA's `null` takes a period at once and the pipe is the buffer — and the pipe's
figures with it are RFC 0015 section 9.1's.

## Why no priority

This machine grants none to the agent user: `RLIMIT_RTPRIO` is 0, `RLIMIT_NICE` is 0 (no nice
below 0), and no RealtimeKit daemon is on the system bus to ask. On a desktop the default device is
a plug-in (`plug`, or PipeWire's and PulseAudio's `ioplug`), and `cpal`'s `realtime` feature itself
refuses to promote a thread on those types (`is_rt_eligible`: `PLUG`, `IOPLUG` and the rest "can
chain" to a server and invert priority), so the feature would add a dependency and a D-Bus call for
no promotion on the devices the helper meets. The period is the lever the helper has.

## Consequences

- A device whose period range does not hold 20 ms is given its own choice, as before; one whose
  buffer cannot hold two such periods refuses the stream with `cpal`'s sentence, which the window
  says.
- Revisit if the helper is measured on a desktop's server-backed device and underruns there.
