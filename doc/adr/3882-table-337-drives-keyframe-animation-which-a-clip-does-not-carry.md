# 3882 — Table 337 drives keyframe animation, which a sound or video clip does not carry

Session 2295. Status: **accepted and built**. Code: `pdf-model`'s `media/rich_media/views.rs`
(`animation`, `Animation::set_aside`), `media/rich_media.rs` (`activated`). Clauses: ISO 32000-2
§13.7.2.2.4, Table 337, Table 313 (§13.6.3.2). Plan: `doc/todo/80` section 1 step 5. Question:
`doc/questions/Q1116`.

## Context

`doc/todo/80` planned step 5 as "Table 337's `PlayCount` and `Speed` on the players' clock". Read
whole, the clause says what the dictionary drives. §13.7.2.2.4: it "specifies the preferred method
that interactive PDF processors should use to apply timeline scaling to keyframe animations", and
"A keyframe animation can be provided as part of 3D model content". Table 335's `/Animation`:
"describes the preferred method that interactive PDF processor should use to drive keyframe
animations present in this artwork". Table 337's `/Subtype` defers to Table 313, whose styles are
formulas over "keyframe settings specified in the 3D artwork": `[r0, r1]` "is the keyframe animation
time line", and §13.6.3.2 defines a keyframe animation as one that "provides key frames and
specifies the mapping for the position of geometry over a set period of time".

## Decision

1. **The dictionary is read whole, as the table states it**: style `None`, `Linear` or
   `Oscillating`, an unlisted style read as `None` ("an animation style of None shall be used"),
   `/PlayCount` default -1 (a negative count repeats without end), `/Speed` default 1 and positive,
   both "ignored for an animation subtype of type None". A malformed count or speed is said and its
   default used.
2. **A `Sound` or `Video` clip carries no keyframe animation**, so there is no `[r0, r1]` for a
   `Linear` count to repeat or an `m` to scale: the clip plays on its own time line, and a stated
   `Linear` or `Oscillating` animation is said in `Clip::unhonoured` with its count and speed. The
   `3D` configuration's keyframe animation is the excluded artwork's.
3. **Mapping the count onto a clip's repeats and the speed onto its rate is not taken.** It would
   read a clip's time line as Table 313's keyframe time line, which the table does not say;
   `Oscillating`'s cosine time would have no counterpart in a clip at all. Q1116 asks the owner
   whether he wants it as a documented choice; it blocks nothing.

## Consequences

Step 5 is a reading with no player change, and the 13.7.2.2.4 row stays `out-of-scope` on the 3D
artwork exclusion, its note quoting Table 335's "keyframe animations present in this artwork".
