# Round 2189 — the sound streamed to the helper, and held there once

Q1006's option 2, first round: review 2185's three shortfalls removed; A988's MP4 check. ADRs 3457–3460.

## What it did

- **Start** (ADR 3457). The media worker's client hands each run on as it is answered
  (`decode_streamed`, `decode_wave_streamed`, `decode_mp4_streamed`, `Run`); the whole-sound calls
  gather the same runs. The player converts each run with an incremental `Converter` (sample for
  sample the whole conversion, tested over five rate and speaker cases and four run lengths), levels
  it, cuts the section, appends it to the sink, and starts the first pass at the first frames. Every
  bound stays: `at_most` refuses a sound of known length before it sounds; an MP4 clip of unstated
  length is bounded at the run that passes. Passes are provisional until `Passes::learned`.
- **Repeat** (ADR 3457). The wire's whole-sound `PLAY` became `SOUND`/`SAMPLES`/`COMPLETE` and a
  `PLAY` of a pass of the sound the helper holds once (`quorra_sound::Held`); a pass that reaches
  samples still to come writes silence and counts nothing. The window keeps no samples.
- **Resume** (ADR 3458). `PAUSE`/`PAUSED`: the helper answers the frame the device reached — frames
  written less `cpal`'s stated playback delay — and `MediaClock::reached` puts the clock there.
- **Measured** with `tools/audio-in-use.sh` on 2185's pipe and, new, on `snd-aloop`'s loopback
  card (`AUDIO_IN_USE_ROUTE=loopback`, ADR 3459, run under `sg audio`). Figures: RFC 0015 §9.1 and
  review 2185's round-2189 section, dated beside the first.
- **A988** (ADR 3460). A Sonnet reading of `mp4.rs` against the fourth edition found no contradicted
  reading; three comment sentences now cite the held edition (a size-0 box, `AudioSampleEntryV1`
  belongs to later editions, the zero segment duration's ground).

**Gates.** Over `-p viewer-host -p pdf-media-worker -p quorra-sound -p viewer-qt -p viewer-gtk -p
viewer-ui -p viewer-core`: fmt exit 0; clippy `-D warnings --all-targets` exit 0; `pdf-sandbox --bins`
exit 0; nextest `--lib --bins --tests` (`xvfb-run -a`) 1429 passed, 0 failed, `audit-diff.py` "failed
0, expected 0"; rustdoc `-D warnings --document-private-items` of `quorra-sound`, `pdf-media-worker`,
`viewer-host` exit 0; `fuzz` `cargo check --bins` exit 0; `cargo test -p conformance` exit 0.

## Trap
A test double that reports a pass's end the moment it starts is no longer a device once a sound
streams: the real helper ends a pass only when the sound is complete. The doubles take the sound
from a `Held` and the player begins a sound of one run only once it is whole (`Run::last`).

## Left
Underruns: not better on the pipe (25–46 s in the long sound against 2185's 4–16.5 s), about 2.2 s a
minute at every load in run 6, unexplained — the priority-and-buffer round's first question.
