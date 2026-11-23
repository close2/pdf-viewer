# 3459 — The sound path is also measured on ALSA's loopback card

Session 2189. Status: accepted. Context: ADR 3441 (the pipe stand-in); the owner loaded `snd-aloop`
and put the agent user in `audio` on 2026-11-23; RFC 0015 section 9.1.

## Decision

`tools/audio-in-use.sh` keeps 2185's route — ALSA's `file` plugin over `null`, into a one-page pipe
read on the monotonic clock — and gains `AUDIO_IN_USE_ROUTE=loopback`: the helper's default device
is `hw:Loopback,0,0` (48 kHz stereo `S16_LE`, the format the capture opens first and the loopback
then holds the playback to), and `ffmpeg` captures `hw:Loopback,1,0` as floats into the test. The
run goes through `sg audio` where the shell does not carry the group. Both routes' figures are
recorded.

## What the loopback measures, and what it does not

It is a kernel device with its own timer, period and buffer, which the driver enforces: what the
pipe could not show — a driver's buffer between the helper and the frames, real period wakeups, and
ALSA's own delay figure, which ADR 3458's resumption reads. It is still no card. There is no
converter and no crystal: the loopback's timer runs on the machine's clock, so drift is again this
program's counting. There is no speaker and no latency to one. Its frames reach the test a capture
period late (about 12 ms here, in `ffmpeg`'s reads of 128 frames), so every time read from it is
late by up to that. An underrun there is silence inside a sound whose frame indices go on in order,
not a read that waited, and it is counted so.
