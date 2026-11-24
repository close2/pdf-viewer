# 3593 — A clip's pictures are cut by its section and its duration, on the sound's clock

Session 2223. Status: accepted. Context: RFC 0015 section 6; ADRs 3341, 3398, 3538, 3555; ISO
32000-2 §13.2.4.3, §13.2.5 Table 291, §13.2.6.3 Tables 289–291 and 300.

## Decision

1. **One timeline per clip.** `viewer_host::video::timeline::Timeline` holds, in time, what
   `sound::passes` holds in a device's frames: the section's beginning on the movie's timeline,
   its length where it states an end, Table 291's `/D` (intrinsic, infinite or stated), the
   passes `/RC` still owes, and the fraction of a whole pass the one in hand lasts.
2. **Each pass** decodes from the section's beginning (`MediaWorker::decode_mp4_video_from`,
   which starts at the sync sample before it, ADR 3555); a picture's time in its pass is its
   movie time less that beginning. A picture at or past the cut — the section's end, or the
   pass's length where that is shorter — is not shown; the pass's pictures end there and the
   decode is stopped.
3. **A pass ends** at its length on the clock: a stated duration's share (the last picture held
   up past the pictures' end, §13.2.5's "freeze the media in its final state"), the intrinsic
   duration's once the first pass has taught it, never for an infinite one. The next pass begins
   exactly there — `pass_start` advances by the pass's length, not by when the window happened
   to look — so a picture and the sound's pass boundary agree.
4. **A fractional count** cuts the last pass as the sound's is cut. Under the intrinsic duration
   a count below one has a first pass whose length only its own decode teaches; its pictures are
   shown whole and the sentence starting the clip says so.

## Consequences

`Shown::cut` and its caveat "its pictures play whole" are gone. The test
`the_pictures_and_the_sound_agree_at_every_cut` runs `sound::passes` and the pictures' timeline
over one section, duration and count and compares the instants every pass ends; the player's own
tests cut, freeze and repeat against a queue, and `played_video.rs` cuts a real decode. Not
measured: a loopback beep against a picture at a cut (`audio_in_use.rs` measures the slaving,
ADR 3556, not the cut).
