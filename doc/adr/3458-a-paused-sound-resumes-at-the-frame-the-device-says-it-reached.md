# 3458 — A paused sound resumes at the frame the device says it reached

Session 2189. Status: accepted. Context: review 2185 ("A resume repeats up to a report's span");
ADRs 3342, 3433, 3457.

## Decision

A pause is an order of its own on the helper's wire (`PAUSE`), answered with `PAUSED` and the frames
of the pass the device had reached. The helper reads that from its output stream's last callback:
the frames written before it, plus the time since, less the delay `cpal` states for it (the
callback's playback instant less its callback instant, the frames the device still held), held
between what had been written and nothing. The player gives that count to the media clock
(`MediaClock::reached`), which replaces the reports — they count frames written, which run ahead of
those heard — and the resumption starts there.

A sink that cannot say (`Sink::pause` answers `None`: a test's double, a helper that did not answer)
leaves the resumption where the reports put it, as before. The ALSA delay is the device's own word:
where a device under-reports what it holds the resumption skips those frames rather than repeating
them, and the measurement in RFC 0015 section 9.1 reads which.
