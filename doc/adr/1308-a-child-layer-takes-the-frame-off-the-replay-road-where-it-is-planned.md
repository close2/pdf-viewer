# 1308 — A child layer takes the frame off the replay road where it is planned

Status: accepted. Session 1235.
Amends: raster's ADR 0087 (record replay) in where its admission rule is enforced.

## The defect

`encode/replay.rs` states the admission: a frame that meets a child layer "is abandoned", and
"the sites that build those structures are the sites that abandon the list". `record_fill`'s
comment relied on it ("a blended fill re-enters through a child layer, which takes the whole
frame off the replay road before the copy could lie"). The code did not: the group arm and
§11.7.4.3's layer called `unreplayable()`, but `fill_through_blend_group` — the implicit group
§11.3.5 puts a solid fill with a non-Normal blend in — did not. Such a fill wrote two
`SolidFill` records (the outer call's and the inner one's), both with the Normal style, and a
zoom step replayed them as two plain fills. `raster-gpu/tests/record_replay.rs::
a_frame_with_a_blended_fill_re_encodes_instead_of_replaying` measured it before the fix: the
frame came back `RecordReplayed`, with 2298 of 9216 pixels differing from the walk.

## The verdict and the fix

The comment is right and the code was wrong. `unreplayable()` is now called in `plan_child`,
the one function every child layer is planned through — groups, the implicit blend groups of
fills, strokes and images, §11.7.4.3's layer, a soft mask's plan — so the admission is
structural rather than a promise at each call site; the two explicit calls it subsumes are
gone. The test above holds it.

## The cost, written down

A stroke or an image under a non-Normal blend used to stay on the replay road through its
`Slow` record, which re-dispatches the command and so rebuilt its layer correctly. It now
re-walks, like every other frame with a layer. Nothing measured that case's replay, and a rule
enforced in one place is worth more here than a saving nobody counted.
