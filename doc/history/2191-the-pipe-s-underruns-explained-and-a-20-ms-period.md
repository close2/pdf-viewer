# Round 2191 — the pipe's underruns explained, and a 20 ms period

Q1006's option 2, second round: the open findings of round 2189. ADRs 3465–3468.

## What it did

- **Traced** (ADR 3465). `QUORRA_SOUND_TRACE=<dir>` makes the helper record each callback (wall
  instant, gap, read and lock times, frames asked, given and arrived, `cpal`'s delay) into a buffer
  allocated before the first, written per pass at the drop; unset, no clock is read for it.
  `AUDIO_IN_USE_TRACE=1` sets it and the test writes its underruns and late reads beside it.
- **The cause**: each long pipe underrun had a helper gap of the underrun plus 15.7 ms beside it,
  the callbacks microseconds; a `/proc` sampler put the stream thread in `D` at
  `do_get_write_access` — the pipe's node on the scratch disk's ext4, waiting for the journal's
  five-second commit (20–670 ms). The node is now on tmpfs: the longest underrun went from 655 to
  26.5 ms. `Held::append` copies a run before taking its lock.
- **Period** (ADR 3466): `BufferSize::Fixed` of 20 ms where the device's range holds it. Loopback
  silences in the long sound 291 (1.49 s) → 0. No priority: `RLIMIT_RTPRIO` 0, `RLIMIT_NICE` 0,
  no RealtimeKit, and `cpal`'s `realtime` refuses plug-in devices anyway.
- **Loopback pairs** (ADR 3468): `snd-aloop` copies the stale ring at a late application.
- **Resume** (ADR 3467): the device's delay is kept; pausing the device first was measured and
  refused (the pipe's pause tail went to 97–573 ms, unexplained).
- Readings: RFC 0015 section 9.1 and review 2185's round-2191 section.

**Gates.** Over `-p quorra-sound -p viewer-host -p viewer-qt`: fmt exit 0; clippy `-D warnings
--all-targets` exit 0; Windows clippy of `quorra-sound` exit 0; `cargo deny check` exit 0;
`pdf-sandbox --bins` exit 0; nextest `--lib --bins --tests` (`xvfb-run -a`) 492 passed, 0 failed, `audit-diff.py`
"failed 0, expected 0"; rustdoc `-D warnings --document-private-items` of `quorra-sound`, `viewer-host` exit 0;
`cargo test -p conformance` exit 0.

## Trap
A stand-in device's file system is part of it: a named pipe on a journalled disk stalls its writer
at each journal commit, and that measured the disk for two rounds. Put an instrument's nodes on
tmpfs, and trace both ends on one clock before reading a figure as the program's.

## Left
1. The pipe's pause tail when the device is paused first (97–573 ms, outside the helper).
2. A desktop's server-backed device (PipeWire, PulseAudio) is unmeasured.
3. The pipe's remaining underruns follow the load (its one-page buffer is empty each 10.7 ms).
