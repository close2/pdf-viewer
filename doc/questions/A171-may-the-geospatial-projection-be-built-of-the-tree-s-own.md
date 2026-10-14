Status: complete
Given: 2026-10-05, in conversation — transcribed by the round
Owes: the projection built — the census over the crawl as the round's first commit (the `/WKT`-or-`/EPSG` fork it answers deciding whether any registry is needed at all); the `/WKT` fork taken as far as the files allow, parameters inline; the inverse methods the census finds, each formula cited to IOGP Guidance Note 7-2's own section with its worked examples as the test source; WKT1 carried only if the census finds it; an accuracy budget documented as a choice; `/EPSG` codes answered from tables of just the codes documents use, the rest refused by name; and if the census finds no documents at all, the round says so and the build is coverage taken on this answer rather than re-asked (acting round)

> I agree with all your recommendations.

Reading: the recommendation of the `Q` file, adopted as written. The census is the build round's
first commit and not a gate: it exists because it can delete the expensive half of the scope — if
the population carries `/WKT`, the registry is not needed at all — and because it fixes the method
set, the dialects and the beneficiary count in one pass, over the crawl and not the oracle's
documents, with the curated corpora beside it as the control. The honest downsides travel with
the build as obligations: there is no oracle for any of it, so the tests are pinned to Guidance
Note 7-2's formulas and worked examples from the first commit, and the accuracy budget is a
documented choice. The scope is §12.10's own ask — inverse projection within the file's own
datum, no datum transformations — and a crawl of zero documents routes to coverage taken on this
answer, not to "do not build".