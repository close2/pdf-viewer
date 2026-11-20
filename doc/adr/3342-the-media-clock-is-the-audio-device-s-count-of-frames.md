# 3342 — The media clock is the audio device's count of frames

Session 2160. Status: accepted. Context: RFC 0015 section 5 (step 6), ADRs 3317, 3341.

## Decision

1. **The master is the device.** A sound's time is the frames the audio device has taken divided by
   its rate. The sound helper's output callback counts the frames it writes and reports the count —
   a new reply, `PLAYED` (serial, frames), beside `ENDED` — whenever it has moved a twentieth of a
   second's frames, and at the end. `quorra_sound::Progress` (`Played(frames)`, `Ended`) is what a
   device reports through `quorra_sound::Report`, and `viewer_host::sound::Sink::start` takes the
   same report in place of the one-shot `ended` it took.
2. **`viewer_host::sound::clock::MediaClock`** holds one sound's time in the device's frames: the
   passes already over, the device's count in the pass now sounding (a count below one already read
   moves nothing — the clock never runs backwards), and a freeze. `Player::position` reads it.
3. **A freeze is the one span the device cannot count**: §13.2.5's frozen tail writes nothing.
   During it the clock runs on the monotonic clock from the instant the device's count stopped, and
   when it ends the clock moves by exactly the freeze's frames. The player's thread waits for that
   instant with `recv_timeout`, so an order still reaches it during a freeze.
4. **Why the device and not the processor's clock**: the two crystals differ, and an underrun stops
   the device and not the processor. A video timed against the processor's clock would drift from
   its sound by both; timed against the device's count it shows the frame of what was heard. A
   pass's cut stays sample-exact arithmetic on the samples (ADR 3341); the clock is what a freeze,
   a position and a later video read.

## Proof

`viewer-host`'s `sound::clock` unit tests (the count is the time; a freeze counted at the device's
rate and closed exactly); `tests/play_parameters.rs`' `the_clock_is_the_device_s_count`, where the
recording double reports two of four frames at 8000 Hz and the position reads 0.25 ms, and
`a_duration_longer_than_the_sound_is_frozen_silent`; `quorra-sound`'s `tests/pipe.rs`, where the
counts cross the real pipe as 25 000 and 50 000 frames of a 100 000-sample stereo sound. No person
has heard a sound through this path: this machine has no sound card.

## Cost

Twenty replies a second on the helper's pipe while a sound plays (eighteen bytes each), one lock
per report on the player's side; nothing at launch.
