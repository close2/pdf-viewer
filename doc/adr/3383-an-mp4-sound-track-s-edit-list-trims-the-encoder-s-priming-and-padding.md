# 3383 — An MP4 sound track's edit list trims the encoder's priming and padding

Session 2170. Status: accepted. Context: RFC 0015 section 5 (step 9), ADRs 3369, 3381, 3382;
`doc/questions/Q988` (ISO/IEC 14496-12 is not held, so its sentences are paraphrased and cited by
section, never quoted).

## Context

An AAC decoder's first output precedes the first sample the encoder was given — the encoder's
priming, usually one frame (1 024 samples) or 2 112 — and its last frame runs past the last sample.
An MP4 file states both in the track's edit list: ISO/IEC 14496-12, 8.6.5 (`edts`) and 8.6.6
(`elst`). Each entry presents, for its segment duration in the movie's time scale (`mvhd`, 8.2.2),
the media from its media time on, in the media's time scale (`mdhd`); a media time of −1 is an empty
edit, presenting nothing for its duration; each entry has a 16.16 rate, 1 for ordinary play and 0
for a dwell. A track with no list presents its media from its start. Version 1 widens both times to
64 bits.

## Decision

1. **`mp4` reads `mvhd`'s time scale and `trak/edts/elst`** (both versions) into
   `SoundTrack::movie_time_scale` and `SoundTrack::edits`, at most `MAX_EDITS` (1 024) entries.
2. **`mp4::presentation` applies the shape a sound track has**: any leading empty edits become a
   delay of silence, then one edit at rate 1 skips its media time and keeps its segment duration,
   each moved to the media's clock (`mp4::rescaled`, to the nearest unit). A segment duration of 0 is
   read as "the rest of the media" — **a choice**: no held text says what a zero-length edit means in
   a file that is not fragmented, and dropping the whole sound is the worse reading.
3. **Any other list is not applied, and said**: two stretches of media, a rate other than 1, or a
   list that needs a movie time scale the file does not have. The media plays whole and the note
   reaches the window's sentence. A list a player cannot honour is information about the file, not a
   reason to play nothing.
4. **The host trims** (`client::presented`): the worker states the figures in the media's clock,
   and the host moves them to the output's rate once the first run has said it — an HE-AAC stream's
   output is twice its core rate, and a file may state either as its media's time scale. The host
   stops sending runs once the kept length is decoded, so the encoder's padding is never decoded.

## Consequences

- An iTunes-style file (one edit, media time 2 112) and an encoder's own (one edit, media time
  1 024) play from their first real sample and stop at their last.
- The held ISO/IEC 14496-26 streams carry no edit list; their references begin 2 048 samples into
  the decoded output (ADR 3382's walk prints the lag), which is the reference's choice and not one
  this player makes for a file that does not state it.
- Q988's text check covers this paraphrase too.
