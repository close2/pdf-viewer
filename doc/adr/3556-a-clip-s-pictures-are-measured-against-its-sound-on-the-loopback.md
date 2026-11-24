# 3556 — A clip's pictures are measured against its sound on the loopback

Session 2213. Status: accepted. Context: RFC 0015 section 6 round 9 and section 9.1; ADRs 3441,
3459, 3555.

## Decision

`tools/audio-in-use.sh` makes a 120 s clip with `ffmpeg` — `testsrc2` at 25 pictures a second in
H.264 Main with two B pictures (so its composition offsets and edit list are read), and AAC LC
beeps, 50 ms of 1 kHz starting at each whole second — so picture `25 n` and beep `n` start at the
same second of the file. `viewer-host`'s `a_clip_s_pictures_measured_against_its_sound` plays it
through the sound player on the device and the video player beside it, polling `due` each
millisecond as a window would; each second's picture is timed when it is handed on, each beep
when the device took its first frame past a quarter of full scale, and each beep is paired with
the nearest second. `AUDIO_IN_USE_ONLY=av` runs it alone; `AUDIO_IN_USE_AV_UNSLAVED=1` is its
control, the pictures on the monotonic clock as before ADR 3555.

## Readings (2026-11-24, `snd-aloop` card 2 through `sg audio`, four lanes building beside it)

| run | pictures, dropped | picture after sound, median (min, max) | drift, first to last tenth |
|---|---|---|---|
| slaved (ADR 3555) | 3000, 0 | −8.0 ms (−31.5, 46.6) | +3.2 ms |
| slaved, earlier | 2997, 3 | about −7 ms (−38.6, 274.9) | −8.2 ms |
| control, monotonic | 3000, 0 | −113.5 ms (−139.3, −106.8) | −14.2 ms |

The control is the sound's start-up — the helper opening its stream and the device's buffer —
which pictures on their own clock run ahead of; following the heard count removes it. The
remaining few milliseconds early is within the loopback capture's period, by which every beep is
read late. The outliers of the earlier slaved run are pictures handed on late by a polling thread
on a loaded machine (the three it counted dropped), not a moved clock: the next second is back.

## Consequences

The loopback's clock is the machine's, so this measures the slaving and the start, not two
crystals' difference; a speaker's device would add that, which the heard count follows by
construction. A window's present after `due`, one frame at most, is not in the reading.
