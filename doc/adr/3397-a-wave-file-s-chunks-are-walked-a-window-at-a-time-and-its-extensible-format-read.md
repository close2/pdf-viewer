# 3397 — A WAVE file's chunks are walked a window at a time, and its extensible format is read

Session 2174. Status: accepted. Context: ADR 3349 (the WAVE header read in the media worker), ADR
3381 (an MP4 file walked box by box); the RIFF form of 1991, RFC 2361, and Microsoft's
*Multiple Channel Audio Data and WAVE Files*, none held in `doc/`.

## Decision

1. **The layout request carries a window and where it begins.** `FRAME_LAYOUT` is the version,
   the file's length, the window's offset, the format read so far (`DESCRIBED_BYTES`), then at
   most `MAX_RUN_BYTES` of the file from the offset. The worker answers `FRAME_PLACED` (the
   format, the samples' offset and length, and the channel mask) or `FRAME_ONWARD` (the first
   chunk header the window did not hold whole, and the format so far). The worker stays stateless
   between requests: the format rides the request, as an MP4 walk's position does.
2. **The host checks every step**: an onward offset must lie past the window's own and inside the
   file (`decode_onward`), and a walk stops at `MAX_WAVE_WINDOWS` (1 024) windows, the bound
   `MAX_TOP_LEVEL_BOXES` sets on an MP4 walk. A worker that answers otherwise is distrusted.
3. **A file whose header fits 256 KiB is still read in one exchange**; the walk costs a request
   only for a chunk that ends past a window. ADR 3349's refusal of a header past the first window
   is withdrawn.
4. **`WAVE_FORMAT_EXTENSIBLE`** is read through its `SubFormat` GUID: one ending in the base
   `-0000-0010-8000-00AA00389B71` names the short-form tag in its first bytes, and PCM, IEEE float,
   A-law and µ-law are read as the short form would be; any other GUID is refused with a sentence.
   `wValidBitsPerSample` above the container is refused; below it, the samples are their
   container's, as a twelve-bit short-form sample already was.
5. **IEEE float** (tag 3, short or extensible) is decoded at 32 bits as `Encoding::Float`, scaled
   as the AAC decoder's floats are (`aac::top_aligned`); 64-bit floats are refused by name.
6. **The channel mask is carried and reported, not applied.** The sink places a channel by its
   order in a frame; where the mask is neither absent nor the first speakers one per channel, the
   decode's notes say so, so a file whose channels name other speakers is never misplaced silently.

## Cost

One more frame kind and a wider layout request; the fuzz target `media_sound` checks both replies.
