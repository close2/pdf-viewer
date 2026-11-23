# 3457 — A sound is sent to the helper a run at a time, starts at its first run, and is held there once

Session 2189. Status: accepted. Context: review 2185 ("A sound's start grows with the sound", "The
helper holds two passes at once"); RFC 0015 section 9.1; ADRs 3317, 3341, 3342, 3381, 3397, 3441.

## Decision

1. **The worker's runs are handed on as they are answered.** `MediaWorker::decode_streamed`,
   `decode_wave_streamed` and `decode_mp4_streamed` give each run (`pdf_media_worker::Run`) to the
   caller before the next request; `decode`, `decode_wave` and `decode_mp4` gather the same runs and
   answer what they always did. Every bound the host checks is unchanged — ADR 3381's runs of access
   units and movie bound, ADR 3397's windows, `PROCESSOR_BUDGET` per request — and a run carries
   the sound's length where it is known before decoding (`at_most`: a sound object's and a WAVE
   file's exactly, an MP4 clip's where its edit list states it), so that the player refuses a sound
   past `MAX_PLAYED_SAMPLES` before any of it sounds, as it did. An MP4 clip whose list states no
   length is bounded at the run that passes the bound; where that comes after the sound began, the
   sound is stopped and the sentence says why. The edit list is applied run by run
   (`Presenting`), with the same arithmetic as the whole-clip reading it replaces.
2. **The player converts, levels and cuts each run as it arrives.** `Converter` is the conversion
   of ADR 3301 made incremental: an output frame is written once the input frames it reads have
   arrived and the rest wait for the next run or for `finish`, so a sound converted in runs is the
   sound converted whole sample for sample (`a_sound_converted_in_runs_is_the_sound_converted_whole`).
   Table 291's section is placed in device frames at the first run and cut as runs pass.
3. **The first pass starts at the first frames.** The pass's length is provisional while the sound
   arrives (`Passes` cut as if the sound were endless); once its last run is in, `Passes::learned`
   and `Passes::restated` give the pass in hand and those after it the sound's length, freezing the
   frames a seconds-long duration no longer sounds. A first pass that is a fraction of the sound's
   own length (an intrinsic duration under a repeat count below one) waits for the whole sound, the
   one case whose first cut needs its length; a sound of one run is taken whole.
4. **The helper holds the sound once, and every pass reads it.** The wire's `PLAY` of samples
   became `SOUND`, `SAMPLES`, `COMPLETE` and a `PLAY` of a pass (`from`, `frames`) of the sound
   held (`quorra_sound::Held`). A pass that reaches samples still to come writes silence and counts
   nothing until they arrive, and it ends only where its frames or the complete sound end. A repeat
   is a `PLAY` of the same copy: nothing is sent again and no second pass is read beside the first.
   The window keeps no samples at all.

## Consequences

- The wait before a sound is one run's decoding and conversion, not the whole sound's; review 2185's
  0.3–3.1 s for thirty seconds is re-measured in RFC 0015 section 9.1.
- The helper's resident memory for a sound is one copy (61 MiB of `f32` for 160 s), and the window's
  is a run's.
- Orders that arrive while a sound is still arriving are read between runs: one that ends it (a stop,
  a newer sound, its document closing) ends the decoding there; any other is obeyed at once.
- A note the worker finds only at the end (concealed AAC units) is said after the `Playing` sentence,
  as `Heard::Unchanged`.
