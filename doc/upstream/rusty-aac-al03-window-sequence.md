# rusty_aac: al03_48 differs from its reference where the window sequence changes out of order

Status: **drafted, not filed** (ADR 3400). The text below the line is the issue as it would go to
`rusty_aac`'s tracker; a person files it.

**What is relied on.** ISO/IEC 14496-3 is not held, so this text quotes no clause of it and claims
no reading of it: it reports a difference from the conformance suite's own reference output, and
asks the question. The stream and the reference are ISO/IEC 14496-26's, from ISO's Publicly
Available Standards. The window sequences were read from each access unit's `ics_info` by a probe
outside the tree (`window_sequence`, the two bits after `ics_reserved_bit`); the pairing of
differing blocks with those transitions is a correlation over the whole stream, and no patched
decoder has confirmed it.

---

**Title**: al03_48 (14496-26 conformance) differs from its reference in blocks whose window
sequence does not follow the previous one

**Version**: `rusty_aac` 2.1.1, `default-features = false`, raw access units from the MP4 file
into `decode::Decoder` with the file's `AudioSpecificConfig`.

**Stream**: `al03_48.mp4`, ISO/IEC 14496-26:2010 bitstreams, DVD1
`mpeg4audio-conformance/compressedMp4/`; reference `al03_48.wav` in `referencesWav/` (24-bit PCM).
AAC LC, 48 kHz, mono; each raw data block is a DSE (tag 15, two bytes) and an SCE.

**What differs**: at the lag of 2 048 samples, 361 of 386 blocks match the reference exactly
(error RMS 0.00 16-bit LSB) and two more within 0.01 LSB. The other 23 — access units 15, 30, 62, 63, 95, 126, 142, 143, 158,
159, 175, 190, 191, 207, 238, 239, 254, 286, 303, 318, 319, 335, 350 — differ by up to 2 800
16-bit LSB (RMS over the stream 88 LSB). Each of them is a block whose `window_sequence` does not
follow its predecessor in the usual order (ONLY_LONG or LONG_STOP followed by EIGHT_SHORT or
LONG_STOP; LONG_START followed by ONLY_LONG or LONG_START; EIGHT_SHORT followed by ONLY_LONG or
LONG_START). Units 382 and 383, the two within 0.01 LSB, are such blocks too and fall in
near-silence.

**Where**: `src/decode/synth.rs`, `imdct_and_window`, lines 236–249: the left half of the window
is the long slope only where `is_long_end(prev_seq)` and the current sequence is ONLY_LONG or
LONG_START, and the short slope otherwise, so the window applied to such a block is decided by the
previous block's sequence as well as its own.

**Question**: should the left half of the window follow the current block's `window_sequence`
(and its `window_shape` with the previous block's shape) alone, as the reference decoder appears
to? The other 21 held LC streams of the suite decode within one 16-bit LSB of their references.
