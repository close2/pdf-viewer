# Review 2185 — audio's clock and sinks, measured in use

Round 2185, 2026-11-23. A946's third point keeps video closed "until audio has shipped and its clock
and sinks are measured in use". Audio shipped in rounds 2148–2183 (ADRs 3297–3433). This review is
the measurement; `doc/questions/Q1006` asks whether to open video on it. ADR 3441 records the
instrument.

## Method

**The command.** `tools/audio-in-use.sh [<out-dir>]` builds `quorra-sound` and `pdf-media-worker`
in the `gates` profile, makes the AAC LC clip with `ffmpeg`'s own encoder, fetches ISO/IEC
14496-26's two HE-AAC streams from ISO's Publicly Available Standards (pinned by SHA-256, never
committed), and runs `crates/viewer-host/tests/audio_in_use.rs` (ignored, `// not a gate:`) in the
same profile. Every figure below is a line that file prints; `<out-dir>/readings.txt` keeps them.
`AUDIO_IN_USE_LONG_SECONDS=5` is a ninety-second dry run of the same.

**What is driven.** The window's own `viewer_host::sound::Player`: its thread, the confined worker
(decoding under `Profile::Media`), the conversion, and the helper program started through
`helper::open_at`, exactly as a window starts them. The worker and the helper are found among the
process's children (`/proc/self/task/*/children`) and read from `/proc/<pid>/status` (`VmHWM`,
`VmPeak`) and `/proc/<pid>/schedstat` (processor nanoseconds).

**The device.** `aplay -l` is not installed; `/proc/asound/cards` lists an Intel HDA controller
with no playback device and the RX 580's HDMI audio with six, no monitor present on any, and every
`/dev/snd` node is `root:audio 0660` while the agent user is not in `audio`. The helper's own probe
on the default configuration answers "The requested audio device is not available". No sound server
is installed, and `snd-dummy` and `snd-aloop` are kernel modules only root loads. ALSA's `null`
plugin, which a user may open, has no clock: five seconds played into it end in three milliseconds.
So the device is built: the helper's `HOME` names a directory whose `.asoundrc` makes the default
PCM a `plug` to 48 kHz stereo `FLOAT_LE` into a `file` plugin over `null`, writing into a named
pipe. The pipe is shrunk to one page — 512 frames, 10.7 ms, the device's buffer — and a thread
reads 48 frames each millisecond when they are due on the monotonic clock. That reader is the
device's sample clock.

**What the stand-in measures, and what it does not.** It measures everything this program does: the
player's thread, the worker's decoding, the conversion, the wire, the helper, its `cpal` stream and
ALSA's plugins, up to the last byte handed to a device. It does not measure a card: its buffer and
converter (a hardware period is commonly 5 to 25 ms more), its latency to the speaker, or the drift
of its crystal against the processor's clock — the stand-in's clock *is* the processor's, so a
drift figure here is the program's counting, never a crystal's. Its underruns are a 10.7 ms buffer's
(a card or sound server usually holds more), and a read that waited counts both the helper's
lateness and the reader's own wake-up under load, so their count is the helper having fallen more
than a buffer behind and their durations are upper bounds.

**The samples say where they are.** Every measured §13.3 and WAVE sound is 48 kHz stereo, the
device's own format, so conversion is the identity, and each frame carries its index (low sixteen
bits in the left channel, the rest plus one in the right). The reader therefore knows which frame
of the sound it takes at each instant: the resume point, any lost or repeated frame, and the media
clock's error are read off the frames. The AAC clips carry no index; for them the device records
only sound and silence.

**The load.** This machine's normal state: four lanes and the post-merge train building and testing
beside the run, on 32 processors. Each reading prints the load averages at its start and end.

## Readings

Four runs, each the whole command, 2026-11-23, under the load averages printed at each start and end
(32 processors): run 1 358 → 82, run 2 94 → 17, run 3 18 → 34, run 4 37 → 211. Runs 1 and 2 shared
the machine with this round's own gates; run 3 with its second launch gate; runs 1–3 lacked the
per-minute lines, which run 4 added. The session worked in `~/scratch/agent/r2185/`; every figure it read is in the table below.

| figure | command's line | run 1 | run 2 | run 3 | run 4 | bound |
|---|---|---|---|---|---|---|
| activation → first frame at the device, cold (2 s §13.3 sound; helper, device and worker not started), median (min–max of 5) | `cold start` | 144 (104–276) ms | 42 (37–44) | 44 (38–60) | 125 (125–168) | — |
| activation → first frame, warm, median (min–max of 5) | `warm start` | 70 (31–117) ms | 28 (25–33) | 25 (25–28) | 25 (24–30) | — |
| activation → first frame, a 30 s §13.3 sound | `§13.3 raw` | 626 ms | 411 | 372 | 402 | — |
| the same, a 30 s RIFF WAVE clip | `RIFF WAVE` | 3 083 ms | 425 | 718 | 956 | — |
| the same, a 30 s AAC LC clip | `MP4 AAC LC` | 3 033 ms | 336 | 316 | 341 | — |
| the same, HE-AAC `al_sbr_cm_48_2` (32 s) | `MP4 HE-AAC` | 651 ms | 511 | 481 | 820 | — |
| the same, HE-AAC v2 `al_sbr_ps_01_new` (6.8 s) | `MP4 HE-AAC v2` | 304 ms | 517 | 279 | 279 | — |
| pause → last frame at the device (3 trials) | `pause/resume/stop` | 19, 23, 24 ms | 25, 23, 23 | 23, 24, 23 | 20, 24, 24 | — |
| resume → first frame | 〃 | 16, 22, 14 ms | 22, 11, 10 | 9, 12, 15 | 24, 11, 10 | — |
| frames repeated across a pause (resume point − last frame heard) | 〃 | 40, 10, 15 ms | 25, 25, 25 | 25, 25, 25 | 15, 25, 25 | — |
| stop → last frame | 〃 | 18, 19, 19 ms | 23, 23, 25 | 23, 23, 23 | 23, 23, 16 | — |
| media clock − device's frames over the 160 s sound × 4: least-squares drift | `media clock` | 3.7 ppm | 7.5 | −1.8 | 9.4 | — |
| the same: offset range | 〃 | −30 to +26 ms | −30 to +25 | −32 to +24 | −37 to +25 | — |
| frames lost or repeated inside a pass | `frames out of order` | none | none | none | none | — |
| silence between one pass and the next, median (min–max of 3) | `passes at the device` | not read | 80 (60–105) ms | 125 (80–135) | 410 (145–2 620) | — |
| worker peak resident / address space, over the five formats | `worker:` | 3.5–5.4 / 5.7–7.2 MiB | 3.3–5.4 / 5.7–7.2 | 3.4–5.3 / 5.7–7.2 | within the same | 64 MiB address space (`Profile::Media`) |
| worker processor per second of sound, the largest of the five | `worker:` | 8.2 ms (WAVE) | 5.4 (HE v2) | 3.6 (HE) | within the same | 1 s per request (`PROCESSOR_BUDGET`) |
| helper peak resident, a 30 s sound / the 160 s sound | `helper:`, `long sound's helper` | 14.4–15.3 / 120.7 MiB | 14.5–15.2 / 120.1 | 14.5–15.3 / 120.7 | — / 120.3 | none stated |
| underruns in the long sound: count, time | `in the long sound` | 854, 4.0 s | 1 040, 6.5 s | 841, 4.5 s | 2 414, 16.5 s | — |
| underruns per minute against the one-minute load (run 4) | `underruns by minute` | — | — | — | 8–56 at load 10–21; 154 at 35; 300–457 at 91–262 | — |
| the helper's major faults and swapped memory in the long sound (run 4) | 〃 | — | — | — | 0 and 0 KiB every minute | — |
| the stand-in's own rate against the monotonic clock | `the stand-in device's clock` | −3.2 ppm | +23.4 | +21.7 | +21.6 | — |

## What the readings say

- **The clock holds.** Over 643 to 659 seconds the media clock — the helper's count of frames
  written, reported at most twenty times a second (ADR 3342) — stays within −37 to +26 ms of the
  frame the device is taking, and a least-squares fit finds −1.8 to 9.4 ppm in four runs: at most
  6 ms over the whole run, inside the report's own 50 ms step. No frame is lost or repeated inside a pass. The jitter is the report's
  granularity plus the frames between written and taken, which is exactly what ADR 3342 accepted.
  Drift of a real card's crystal is not in this figure (Method).
- **The worker is far inside its budgets.** Its peak address space, 7.2 MiB at most, is a ninth of
  `Profile::Media`'s 64 MiB; its processor time per second of sound is at most 8.2 ms, so a request
  of 32 HE-AAC access units (2 048 frames each at 48 kHz, 1.37 s of sound) costs about 12 ms
  against a budget of one second.
- **A sound's start grows with the sound.** The player decodes, converts and sends a sound whole
  before its first frame: 24 to 33 ms warm and 37 to 60 ms cold for two seconds when the machine is
  quiet enough, 0.3 to 3.1 s for thirty seconds, and the `Playing` sentence and the first frame are within a millisecond
  of each other for the raw and WAVE sounds — the device starts at once, the wait is the whole-sound
  preparation. For a 170-second sound, the most `MAX_PLAYED_SAMPLES` admits, it is several seconds.
- **A resume repeats up to a report's span.** The resume point is the device's last report, which
  lags the frames written by up to 50 ms; the readings show 10 to 40 ms repeated.
- **The helper holds two passes at once.** A repeat is a new `PLAY` of the pass's samples, read whole
  before the stream of the one before is dropped: 120 MiB resident for a 160-second sound (61 MiB of
  `f32` twice), and 60 ms to 2.6 s of silence between passes while the next is read — longer the
  busier the machine.
- **The helper's stream runs short, and the load decides how often.** In the long sound, 841 to
  2 414 underruns, 0.6 % to 2.5 % of its time, the longest 0.2 to 0.8 s. Run 4's per-minute lines
  rule out paging (no major fault, nothing swapped) and follow the load: 8 to 56 a minute at load
  averages 10 to 21, 300 to 457 at 91 to 262 on 32 processors. The thirty-second sounds, played
  mostly at lower load, had few. The helper's thread asks for no scheduling priority and the
  stand-in's buffer is 10.7 ms; a card or a sound server holds more, so a real device would turn
  some of these into nothing — but not the ones of half a second.
- **The launch path carries no sound.** `Player::new` starts nothing; the one thing a window does at
  launch is look for the helper's file (`helper::missing`, ADR 3417). The window binaries link no
  audio library (`ldd` names `libasound` only for `quorra-sound`). `tools/state.sh --round 2185
  launch` was run before (exit 101: one figure below its band, `opt_demo.pdf`'s allocated
  high-water 2.891 MiB under 3.500, on the unchanged tree) and after (exit 0, none outside), drawing
  the same six frames; this round's change touches no code a window runs.

## What is left unmeasured

A card's buffer, converter and latency to the speaker; a crystal's drift; a sound server's own
mixing (none is installed here); the Qt window's `QAudioSink`, which is not this path (the Qt window
plays through the same helper since ADR 3317, so nothing separate remains for it); and Windows,
whose window plays through the same helper and has not been run here. A machine with a card the
user may open measures the first three by pointing the wrapper's configuration at it.

## Readings after the sound was streamed — round 2189, 2026-11-23

Round 2189 removed the three shortfalls above (ADRs 3457, 3458) and ran the same command again: twice
on the pipe (run 5 with the round's own gates building beside it, load 33–151; run 6 with nothing of
its own, load 213 at its start falling to 15), and once on ALSA's loopback card
(`AUDIO_IN_USE_ROUTE=loopback`, ADR 3459; load 16–254). The figures are beside the first table's,
not in place of them.

| figure | runs 1–4 (above) | run 5, pipe | run 6, pipe | loopback |
|---|---|---|---|---|
| activation → first frame, cold, median | 42–144 ms | 56 ms | 114 | 168 |
| activation → first frame, warm, median | 25–70 ms | 33 ms | 65 | 61 |
| the same, a 30 s §13.3 sound / WAVE / AAC LC | 372–626 / 425–3 083 / 316–3 033 ms | 66 / 163 / 31 ms | 172 / 343 / 38 | 281 / 86 / 62 |
| the same, HE-AAC / HE-AAC v2 | 481–820 / 279–517 ms | 460 / 210 ms | 164 / 863 | 200 / 245 |
| frames repeated (−) or skipped (+) across a pause | 10–40 ms repeated | −2.6, 0, 0 ms | −4.7, −2.9, −1.6 | +3.5, +1.1, — |
| media clock drift; offset range | −1.8 to 9.4 ppm; −37 to +26 ms | 1.5 ppm; −45 to +25 | 4.9; −32 to +25 | 7.3; −43 to +116 |
| silence between passes, median (min–max) | 80–410 (60–2 620) ms | 5 (0–5) ms | 5 (0–5) | 0 (0–60) |
| helper peak resident, the 160 s sound | 120.1–120.7 MiB | 62.5 MiB | 62.6 | 62.7 |
| underruns in the long sound: count, time | 841–2 414, 4.0–16.5 s | 3 178, 46 s | 714, 25 s | 768 silences, 4.0 s |

What they say:

- **A sound starts at its first run.** Thirty seconds now start in 31–343 ms on the pipe, the slower
  figures at load averages near 200; the whole-sound wait of 0.3–3.1 s is gone. The HE-AAC clips'
  first frames trail `Playing` by about their own leading silence — 136 and 184 ms of zeros, by
  `ffmpeg`'s decoding of the same files, evidence only — and by more under load, where the helper
  reached samples still to come (run 6's 863 ms at load 213).
- **A resume repeats or skips under 5 ms**, where it repeated 10–40 ms: the resumption is the frame
  the device said it reached. On the pipe the device's stated delay is a little short of what the
  pipe holds (it repeats up to 4.7 ms); on the loopback a little long (it skips up to 3.5 ms).
- **One pass is held**: 62.5–62.7 MiB for the 160 s sound, its one copy, where two were; the silence
  between passes is 0–5 ms where it was 60 ms to 2.6 s.
- **The underruns are not better, and run 6's are not explained.** Run 6 spent about 2.2 s a minute
  in underruns at every load from 15 to 49, where 2185's runs followed the load; a stall of the
  helper's own would look like that, and none was found by reading. On the loopback, the helper's
  silences were fewer (4–12 a minute at load 16–24) but its 4 462 out-of-order frames come in pairs
  of a 1 024-frame step back and one forward — the loopback replaying a stale period at a playback
  xrun, which `cpal` reported beside them — so its xruns are those pairs too. Both are the next
  round's, with the helper's scheduling priority and its buffer.
- **What the loopback adds and leaves**: a driver's period and buffer and ALSA's own delay figure;
  still no converter, no crystal (its timer is the machine's clock) and no speaker, and every time
  read from it is late by up to a capture period, about 12 ms.
