# Q1006 — May video be opened, now that audio's clock and sinks are measured in use?

Source: round 2185 (ADR 3441), for A946's third point: "Keep **video** closed until audio has
shipped and its clock and sinks are measured in use, then ask before opening §6's 14 rounds."

## The evidence, in one paragraph

Audio shipped in rounds 2148–2183, and round 2185 measured it in use with
`tools/audio-in-use.sh` (`doc/reviews/2185-audio-measured-in-use.md`), on a stand-in device —
ALSA's `file` plugin into a pipe read at 48 kHz on the monotonic clock, because this machine's user
may open no sound card — four times under this machine's ordinary load (load averages from 10 to
360 on 32 processors, four lanes building). **The clock holds**: over a 160-second sound played four times,
about 650 s, the media clock stays within −37 to +26 ms of the frame the device is taking, with no
trend a least-squares fit can tell from that jitter (−1.8 to 9.4 ppm), and no frame is lost or
repeated inside a pass. **The budgets hold with room**: the confined worker peaks at 5.4 MiB resident and 7.2 MiB of
address space against `Profile::Media`'s 64 MiB, and spends at most 8.2 ms of processor per
second of sound against a budget of a second per request of about 1.4 s of sound. **Four things
are not yet what video needs**: a sound is decoded, converted and sent whole before its first frame
sounds, so activation to sound grows with the clip — 24 to 276 ms for a two-second sound, 0.3 to
3.1 s for a thirty-second one; the helper's stream runs short of frames as the load rises, from
8–56 times a minute at load 10–21 to 300–457 at 91–262, up to 0.8 s at a time, with no paging
involved (the stand-in's buffer is 10.7 ms; a card's is usually larger, and the helper's thread
asks for no scheduling priority); a repeat holds two passes in the helper and leaves 60 ms to
2.6 s of silence between them; and a resume restarts from the device's last report, 10 to 40 ms
before where the sound was heard to stop. What the stand-in cannot measure is a
card's own buffer and a crystal's drift against the processor's clock.

## The question

May RFC 0015 section 6's fourteen rounds of video be opened now?

## Options

1. **Open video now, in section 6's order.** Its first two rounds — the streaming arm in
   `confined-transport` and the shared-memory ring — are the construction that also takes audio off
   whole-sound decoding; audio's start latency and the helper's buffer would be fixed inside them.
2. **Open video after two audio rounds** that take the measured shortfalls first: (a) the sound
   streamed to the helper a window at a time, so the first frame no longer waits for the whole clip
   and a repeat neither holds two passes nor falls silent between them, and the resume point read
   from frames heard rather than the last report; (b) the helper's stream
   given a larger buffer and a scheduling priority where the system grants one, re-measured with
   `tools/audio-in-use.sh`. Then section 6's fourteen, on a clock already shown to carry A/V sync.
3. **Keep video closed until audio is measured on a real card**: the owner gives the agent user a
   device it may open (membership of `audio` with an output attached, or `snd-aloop` loaded), and
   the same tool is run against it, so the clock's drift is a crystal's and the latency includes a
   card's buffer. Either of 1 or 2 may follow.

## Recommendation

**Option 2**, with option 3's reading taken whenever a device is available rather than waited for.
The clock — the one thing video's synchronisation rests on — holds on every reading the stand-in can
take; what does not yet hold is a sound's start, its repeat and the helper's resistance to load, and both are
cheaper to fix and to re-measure on audio alone than inside the first video rounds, where a late
frame would be ambiguous between the two media.
