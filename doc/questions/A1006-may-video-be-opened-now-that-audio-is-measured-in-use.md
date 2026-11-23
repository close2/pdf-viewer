Status: complete
Given: 2026-11-23, in conversation — transcribed by the round
Owes: complete two audio rounds (windowed streaming, gapless repeat, heard-frame resume, larger helper buffer and scheduling priority) and re-measure with tools/audio-in-use.sh; then open RFC 0015 section 6's fourteen video rounds; take a real-card or snd-aloop reading opportunistically when a device is available

> Q1006 agree with your recommendation

Reading:
Option 2. Run the two focused audio rounds first to fix the four measured shortfalls — (a) streaming audio to the helper a window at a time so start latency no longer scales with clip length, repeats neither hold two passes nor leave silence between them, and resume uses frames heard rather than the device's last report; and (b) giving the helper stream a larger buffer and scheduling priority where the system grants one, re-measured with `tools/audio-in-use.sh`. Once those land, open RFC 0015 section 6's fourteen video rounds on that verified clock so late-frame issues are not ambiguous between audio and video. Take Option 3's real sound card (or `snd-aloop`) reading opportunistically whenever a device is available rather than blocking video on it.
