# Q1071 — May `personwithdog.pdf`'s turn band be re-taken now that the window draws its black ink?

Source: round 2250, ADRs 3701 and 3702.

## What the tree does now

`personwithdog.pdf` page 1 has a `DeviceCMYK` page group. ISO 32000-2 §11.4.7 makes its composite
four components, so the backends receive two lists, the page's and its black list. Until ADR 3701
the window drew the first and dropped the second, and the page showed no black ink (review 2248,
B02's class). The turn gate's band for the page was taken on that picture.

The window now draws both lists and converts them as `rasterize` does: two device frames, their
read-backs, the conversion and one upload. Round 2250 cut what had no answer: the finishing now
covers only the page's own rectangle of the window, and the conversion is spread across rayon with
identical bytes. Main against the branch, quiet and back to back: turn 31/106 ms, seventh 29/106,
step 38/150 before the cuts. After the cuts the gate measured 80/80/112, at a load past the ceiling
(not judged). The page still fails its band.

## The ask

May that row's `turn`, `seventh` and `step` bands be re-taken from a quiet, judged run of the
fixed tree (ADR 3702 states the procedure and the `why` line), accepting a turn about 2.5 times the
old band for a picture that now has its black ink?

## Recommendation

Yes. The cost left is §11.4.7's: two planes need two device frames until the device draws both in
one. ADR 3702 names that construction, a shader the CPU oracle's single statement would then have
to be kept equal to, as the alternative if the answer is no.
