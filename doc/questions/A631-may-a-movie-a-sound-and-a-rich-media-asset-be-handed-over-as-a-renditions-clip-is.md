Status: complete
Given: 2026-10-26, in conversation — transcribed by the round
Owes: the build — the hand-off for a movie first (Table 306's `/F` handed to the machine's own
player, a movie annotation's activation and a movie action's `Play` through the same hand-off a
rendition's clip uses, `Stop`/`Pause`/`Resume` acting on nothing playing as a rendition's do),
then a sound with `/F` (a self-describing file — AIFF, RIFF, snd — the same hand-off), then rich
media assets (§7.11.4's embedded files reachable from a `RichMediaContent` dictionary walked by
`attachment::attachments`, savable under the existing `EmbeddedDocuments` level, not played);
a sound with embedded samples gets a RIFF wrapper written for the hand-off (µ-law and A-law
across unchanged, 8-bit signed and big-endian 16-bit re-encoded on the way) and Tier B stays on
demand; the rows leave `out-of-scope` as each is built (acting round)

> Otherwise I agree with all recommendations.  (only departures: XFA and Q632, which I haven't
> fully understood yet)

Reading: one message answered six questions together and named its two departures; this one
adopts Q631's recommendation as presented, quoted and attributed: *"Yes to 1, in the order movie,
sound with `/F`, assets — the entry's words already say it, and the hand-off and its level exist.
For 2, the RIFF hand-off with the movie round, and Tier B still on demand."*

What the answer settles:

- **The A254 amendment's "the media file" covers all three**: a movie's `/F`, a sound object's
  `/F`, and a rich-media annotation's assets — each is a file the document holds, and the entry
  puts handing it to this machine's own player in scope. Nothing is played in-process; the
  hand-off's `ask` level governs every one.
- **The rich-media assets are savable, not played** — handing a Flash asset to a player is
  pointless and saving it is not, so they enter through the attachments walk and its existing
  level.
- **Embedded-sample sounds reach the player as files, not as Tier B**: the RIFF wrapper keeps
  every sample and starts nothing in-process; the in-process player stays on demand, which no
  corpus document has yet made.