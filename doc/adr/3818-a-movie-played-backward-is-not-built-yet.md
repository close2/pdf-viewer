# 3818 — A movie played backward, and `Palindrome`'s backward half, are not built yet

Session 2279. Status: **accepted**; the 13.4 row is `partial` until they are built. Code:
`pdf-model`'s `media/deprecated.rs` (`told`). Clauses: ISO 32000-2 §13.4, Table 307. Finding:
2278-D01. Plan: `doc/todo/80-rich-media-sound-and-video-and-a-movie-played-backward.md`.

## Context

Table 307's `/Rate`: "If the value of this entry is negative, the movie shall be played backward
with respect to Start and Duration." Its `/Mode` `Palindrome`: "Play continuously forward and
backward until stopped." The movie this program shows (ADR 3594) is decoded by the confined media
worker forward only, from a sync sample (ADR 3555); both statements were said as set aside while the
13.4 row read `implemented`.

## Decision

1. **Neither is a departure.** Both are `shall`s of a clause in scope, and `CLAUDE.md` forbids a
   clause going out of scope because it is hard. The row says `partial` and names them; the player
   keeps saying each where a document states it — a negative `/Rate` now says it plays forward.
2. **What building them takes**, in RFC 0015 section 6's order (step 12, "Table 307's controller
   bar and modes"): the worker decodes a group of pictures from its sync sample and hands it to the
   ring last picture first, the clock running down a pass; the sound runs backward in the same
   pass, its frames reversed per block; `Palindrome` alternates the two directions. The bound is the
   group: at 1920 by 1080 a sixty-picture group is about 190 MB of decoded planes (4:2:0, 3.1 MB a picture), so the ring holds
   a group and refuses by name a stream whose group is longer than the bound. Estimated at two
   rounds after the rich media plan's first.

## Measured (session 2284)

Step one is built (ADR 3837): the worker holds a group and lets it out last picture first, and a
negative `/Rate` plays the pictures backward in silence. The bound measured is 96 MiB of planes, not
the 190 MB estimated above: a sixty-picture 1080p group does not fit `Profile::Media`'s 256 MiB
address space beside the session's references (a thirty-two-picture group peaks at 174 MiB), so
such a group is refused by name until a round raises the ceiling. The sound's reversal and
`Palindrome` remain.
