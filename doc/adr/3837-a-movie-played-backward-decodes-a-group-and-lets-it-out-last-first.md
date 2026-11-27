# 3837 — A movie played backward decodes one group of pictures and lets it out last picture first

Session 2284. Status: **accepted**. Code: `pdf-media-worker`'s `video.rs` (`Group`,
`MAX_GROUP_BYTES`), `wire.rs` (`VideoRun`), `worker.rs` (`backward`) and `client/video.rs`
(`decode_mp4_video_backward`); `viewer-host`'s `video.rs` (`Backward`) and `sound.rs`; `pdf-model`'s
`media/deprecated.rs` (`told`) and `media/playing.rs` (`Playing::backward`). Clauses: ISO 32000-2
§13.4, Table 307. Plan: ADR 3818, `doc/todo/80` section 2, step 1.

## Context

Table 307's `/Rate`: "If the value of this entry is negative, the movie shall be played backward
with respect to Start and Duration." An H.264 picture can be decoded only forward from a sync
sample — one "from which decoding may start" without what came before it (ISO/IEC 14496-12, 8.6.2)
— so the last picture of a group exists only once the whole group has been decoded.

## Decision

1. **The worker holds one group and lets it out reversed.** A video run carries a direction
   (`VideoRun`): a backward run's first run of a group begins a session of its own from the track's
   parameter sets, every picture the session lets out is held in a `Group`, and the run that ends
   the group lets out the whole of it last picture first, through the same ring, one picture a slot.
   Nothing is let out before the group ends. The worker's system-call set, its ring and its
   descriptor are unchanged (RFC 0015 section 6, A1006): holding a group is heap, no new call.
2. **The host decodes the groups last first and counts the pass down.** A group runs from a sync
   sample to the next; the host sends the last group first. Each picture's time in the pass is the
   distance from where the pass begins — the section's end, or the track's — down to the picture's
   end, so the times rise as the pictures arrive and the window's clock, cut and pass end time a
   backward pass exactly as a forward one. A picture presented at or after the section's end is
   decoded and not shown; a resume skips the groups wholly before its place.
3. **The bound is one group: 96 MiB of decoded planes**, thirty-two 1920 by 1088 pictures. A group
   past it is refused by name — by the host before it is sent where the track's declared size
   already passes it, by the worker as it decodes otherwise — never cut short. 96 MiB is what
   `Profile::Media`'s 256 MiB address space leaves beside a session's references and the ring,
   with the margin measured below.
4. **The sound is the next round's.** Until its blocks are reversed in the same pass, a movie
   played backward is silent: `pdf-model` says so in the clip's notes, the sound player declines
   the sound by name rather than play it forward, and the pictures run on their own clock.
   `Palindrome`'s alternation is said as not built. ADR 3818's plan stands for both.

## Measured

`cargo nextest run -p pdf-media-worker --test video --run-ignored only --no-capture address_space`
(debug build of the worker, three runs, this VM):

| group | planes | first backward picture | worker `VmPeak` |
|---|---|---|---|
| 8 pictures, 32 by 32 (the fixture's size) | 12 288 bytes | 1.5–1.9 ms | 18 280 kB |
| 32 pictures, 1920 by 1088, sixteen references | 100 270 080 bytes | 191–212 ms | 178 440 kB |
| forward, 61 pictures, 1920 by 1088 (ADR 3528's) | — | — | 132 736 kB |

- **The first backward picture waits for the whole group's decode**: at 1080p that is the group's
  decode time (191–212 ms here for intra pictures and skipped copies; a real stream decodes
  slower), on top of the forward start.
- **The peak is 174 MiB, under the 256 MiB ceiling** with 82 MiB to spare; the bound is not raised
  past what that margin carries.
- **The ring needs no resizing**: the group lives in the worker's heap, and the ring still carries
  one picture a slot, four 1080-line slots.
- **What the bound refuses**: a 1080p group of more than thirty-two pictures — a two-second group at
  30 pictures a second is 60, and an encoder's default of 250 is common — while at 1280 by 720
  (1 382 400 bytes a picture) seventy-two fit. Admitting a 1080p two-second group needs about
  188 MB of planes and an address-space ceiling near 512 MiB; that is a measurement for the round
  that raises it, not a choice made here.
