# 3554 — A picture is timed at its composition time, through the video edit list

Session 2213. Status: accepted. Context: RFC 0015 section 6 round 9; ADRs 3526, 3527, 3538;
ISO/IEC 14496-12:2012 8.6.1.2, 8.6.1.3, 8.6.2, 8.6.6 (held, `doc/ISO-IEC-14496-12-2012.pdf`).

## Context

ADR 3538 timed the `k`-th picture out at the `k`-th sample's decoding time: right only where a
track states no composition offsets. 8.6.1.3: "CT(n) = DT(n) + CTTS(n)", unsigned in version 0
and signed in version 1, "each sample must have a unique composition timestamp value". 8.6.6's
media time is "the starting time within the media of this edit segment (in media time scale units,
in composition time)", and the standard's own example shifts a first composition time of 20 to
movie time 0 with one edit of media time 20.

## Decision

1. The worker reads `ctts` (expanded to one offset a sample, refused where it covers other than
   every sample), `stss` (numbers from 1 as indices, strictly increasing, or refused) and the
   video track's `elst` and the movie's time scale, and sends them in the track reply; the host
   checks the reply as before.
2. `client::video::Timing` presents a sample composed at `CT` at `CT − skip + delay`, where skip
   and delay are what `mp4::edited` — the sound track's reading, shared — makes of the list; a
   sample composed before the edit or past its segment is decoded and not shown, and a list of
   another shape is not applied and said (`VideoStreamed::note`).
3. **Which picture is which**: the decoder outputs by picture order count, which a conforming
   stream keeps in composition order, so the `k`-th picture out of a decode begun at sample `s` is
   given the `k`-th smallest composition time of the samples from `s` on.

## Consequences

Tests build the tables by hand over the `I_PCM` stream, so each expected time is arithmetic on
the standard's formula: offsets with and without the shift, an empty edit's delay, a signed
offset composing a picture before the edit, and a two-edit list said and not applied.
