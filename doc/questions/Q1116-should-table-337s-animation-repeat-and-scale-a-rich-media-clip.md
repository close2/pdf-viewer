# Q1116 — Should Table 337's animation repeat and scale a rich media clip, as a documented choice?

Source: round 2295, `doc/todo/80` section 1 step 5 (ADR 3882). Blocks nothing.

## What the tree does now

A rich media activation's `/Animation` (ISO 32000-2 Table 337) is read whole — style, play count,
speed, defaults and the `None` rule — and, for a `Sound` or `Video` configuration, said by name and
not applied: the clip plays once at its own speed. The reason is the clause's own words. Table 335's
`/Animation` "describes the preferred method that interactive PDF processor should use to drive
keyframe animations present in this artwork"; §13.7.2.2.4 says "A keyframe animation can be
provided as part of 3D model content"; Table 313's styles are formulas over "keyframe settings
specified in the 3D artwork", `[r0, r1]` being "the keyframe animation time line". A sound or video
file carries no keyframe animation in §13.6.3.2's sense ("the mapping for the position of geometry
over a set period of time"). The plan had read step 5 as "`PlayCount` and `Speed` on the players'
clock"; the clause does not say that.

## The ask

May a `Linear` animation's `/PlayCount` become the clip's repeat count, and its `/Speed` the clip's
rate, **as a documented choice** rather than a reading? `Oscillating`'s cosine time has no
counterpart in a clip, and its pictures-and-sound playback forward then backward would be a second
choice on top. A clip's rate other than 1 is not built for any player yet (Table 307's `/Rate` of
another size is said the same way), so the speed half would be new player work.

*Recommendation: no.* The standard ties the dictionary to 3D keyframes, and a producer that wants a
rich media video looped has no entry to say so with in PDF 2.0; reading one into Table 337 is the
de-facto convention principle 5 asks us to label rather than derive.
