# 3555 — A clip's pictures follow its sound as heard, and resume from a sync sample

Session 2213. Status: accepted. Context: RFC 0015 section 6 round 9; ADRs 3342, 3349, 3458, 3527,
3538, 3545; ISO/IEC 14496-12:2012 8.6.2.

## Context

ADR 3538 ran a clip's pictures on the monotonic clock beside its sound, which ran on the device's
count: two clocks that start together drift apart by the crystals' difference and the device's
start-up, and the written count runs ahead of the heard sound by what the device holds. A resume
decoded the track again from its start.

## Decision

1. **The device says what it had reached at every report.** The sound helper sends
   `Progress::Heard` — its last callback's written frames less those the device held (ADR 3458's
   arithmetic, now at every report and not only at a pause) — before each `Played`. `MediaClock`
   keeps it and answers `heard_time`: that count run on at the device's rate for at most
   `HEARD_REACH` (100 ms, two reports), so a device that stops taking frames stops the clock.
2. **The pictures follow the sound handed over beside them.** `sound::Player::play` gives each
   sound a ticket; `Player::listening` answers a `Listening` to the last, whose `time` is
   `Waiting` until that sound's pass begins, `At(heard time)` while it is in hand (standing while
   paused), and `Over` once it ended, was stopped or refused, or never sounded.
   `VideoPlayer::play_beside` runs the pictures on it; at `Over` the pictures continue on the
   monotonic clock from the sound's last time, which is also how a silent clip runs throughout.
   `quorra` and the toolkit windows (`Sounds::listening`) hand it over.
3. **A resume decodes from the last sync sample presented at or before the paused place**
   (`MediaWorker::decode_mp4_video_from`), not from the start. A paused decode is not kept waiting
   in the worker: a run's wall budget (ADR 3527) counts the time the host does not read, so a pause
   longer than that budget would be refused as a stall; a sync sample is a place decoding may begin,
   and 8.6.2 says "If the sync sample box is not present, every sample is a sync sample".

## Consequences

- A sound shorter than its pictures hands them to the monotonic clock at its end; repeated passes
  of the two (`/RC`) each start where their own clock stands, so a pass of unequal lengths moves
  them apart by that difference until the next pass.
- A clip waiting for its sound is looked at every 5 ms, since the sound's start wakes no window.
- The measurement is ADR 3556's.
