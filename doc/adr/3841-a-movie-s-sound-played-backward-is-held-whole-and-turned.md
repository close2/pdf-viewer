# 3841 — A movie's sound played backward is held whole and handed to the device turned

Session 2285. Status: **accepted**. Code: `viewer-host`'s `sound/reversed.rs` (`Direction`,
`Turning`, `MAX_REVERSED_SAMPLES`) and `sound.rs` (`Feed::turning`, `Sounding::direction`);
`pdf-model`'s `media/deprecated.rs` (`told`). Clauses: ISO 32000-2 §13.4, Table 307. Plan: ADR
3818, `doc/todo/80` section 2, step 2.

## Context

Table 307's `/Rate`: "If the value of this entry is negative, the movie shall be played backward
with respect to Start and Duration." ADR 3837 built the pictures and left the sound declined. An
AAC frame's samples overlap the frame before it, so the worker decodes a sound forward only, and
the section's last sample exists only once the whole section has been decoded.

## Decision

1. **The player holds the section and turns it, on this side of the worker.** For a movie played
   backward, each run is converted, levelled and cut to §13.2.4.3's section as a forward sound's
   is, and held (`Turning`) instead of handed on. At the last run the held samples are turned —
   the order of the runs and the order within each run reversed, every frame's channels kept in
   their order — and handed to the sink as one sound, which every pass reads (ADR 3457). The
   worker, its protocol and its system-call set are unchanged.
2. **One clock.** The sound's pass starts at the section's end, as the backward pictures' pass
   does (ADR 3837), and the pictures follow the sound's heard time as a forward movie's do (ADR
   3555); the sound is no longer filtered away from the pictures.
3. **The bound is `MAX_REVERSED_SAMPLES`, half of `MAX_PLAYED_SAMPLES`** — 2²³ samples, 87
   seconds of a 48 kHz stereo section — so that `Palindrome`'s two copies (ADR 3842) take no more
   of the sink than one forward sound. Past it the sound is refused by name: before any of it
   sounds where the worker states the length and no section cuts it, and as it is held otherwise;
   it is never cut short. The process holds the section once and the sink once (or twice for a
   palindrome) at the peak, 96 MiB of samples at most.
4. **The cost** is the wait before a backward sound begins: the whole section's decode rather
   than one run's (ADR 3457's streaming start). A movie's AAC decodes far faster than it plays,
   and the pictures wait for the sound to begin, so the two stay together.

## Tests

`played_sounds.rs::a_sound_played_backward_reaches_the_device_last_sample_first` (sixteen samples
carrying their own place reach the device last first, and a section of them is cut and then
turned); `sound::reversed`'s three unit tests (stereo frames across two runs, both palindrome
layouts, the bound by name). Planting `pass_from`'s second copy away fails both palindrome tests.
