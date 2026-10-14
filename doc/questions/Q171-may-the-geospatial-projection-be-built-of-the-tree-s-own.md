# Q171 — May the geospatial projection be built of the tree's own?

Source: this round, on the owner's question of 2026-09-28 — "could we write our own geodesy
implementation?" — sharpened in the same conversation by the owner's challenge: why a census
first, when implementing it would not cost much? This file is the sharpened asking, and its
recommendation no longer makes the census a gate.

## What the clause asks, and what the tree reads already

Everything the *file* states is read: Table 269's measure dictionary, Tables 270 and 271's
coordinate systems, Table 272's point data, the `/GPTS`–`/LPTS` registration pairs and the
`/Bounds` neatline (ADR 1191). What is not done is the one step §12.10's viewport exists for —
turning a projected coordinate into a latitude — and §12.10.3 hands that step to two things
outside ISO 32000-2: an EPSG reference code and a Well Known Text string "specified in ISO 19162".
The rows' own reason for stopping is honesty: "guessing would produce coordinates that look right
and are somewhere else."

## The fork that decides the scope, and the three inputs it chooses between

§12.10.3's Table 270 lets a file state its system one of two ways, and they cost very different
builds:

- **`/WKT` — the parameters come with the file.** A WKT string describes its projection method
  *and all its parameters* inline, so a document carrying one needs only a WKT grammar and the
  method formulas: **no registry at all**. The grammar is ISO 19162's, whose identical text the
  OGC publishes free as its WKT2 standard — holdable under ADR 0187's discipline like any other
  text. (Real files made by older GIS tooling may carry the WKT1 dialect; which dialects the
  population uses is a fact a census states.)
- **`/EPSG` — the parameters live in the registry.** The code names a coordinate system whose
  parameters the IOGP's EPSG Dataset holds — distributed free under its Terms of Use, with
  attribution and no warranty — so this fork needs tables carried from it (the tree's
  compiled-in-static-data rule for the Arlington tables is the precedent) and the maintenance tail
  that comes with them: IOGP revises two or three times a year.

The **formulas are common to both forks**: IOGP's Guidance Note 7 part 2 — a 162-page PDF,
freely downloadable — prints the forward and inverse series for every projection method, with
worked examples where it prints them. **The scope is narrower than "a geodesy library"**:
§12.10 asks for inverse projection within the file's own datum — the coordinate comes back as a
latitude *of the system the file names* — so datum transformation, the deepest water in geodesy,
is not on this path at all.

## What it costs, and the honest downsides

The method math is bounded: inverse-only, five or six families — Transverse Mercator, the Lambert
conformal conics, Albers, the Mercators, Web Mercator — cover the overwhelming share of the
world's projected documents, each an ellipsoid's shared helpers plus one series, cited to
Guidance Note 7-2's own section. With a WKT parser, the whole build is a round or two — the
owner's "would not cost us much" is about right for the `/WKT` fork. The downsides are not size:

- **There is no oracle for any of it.** No renderer in the tree's comparison set implements these
  projections, no pixel gate can see them, and the failure mode is this tree's own words —
  coordinates that look right and are somewhere else — and it applies to our own series
  coefficients exactly as it applied to guessing. The only test sources are the guidance note's
  formulas and its worked examples, and the round must pick an accuracy budget and document it as
  a choice.
- **WKT dialect sprawl**: ISO 19162 is WKT2, but files in the wild were made by tools that emit
  WKT1, and supporting what documents carry is a bigger parser than either dialect alone.
- **The registry tail**, if the `/EPSG` fork is live: attribution, and a dataset that revises two
  or three times a year.
- **Coverage, not robustness, if the population is zero**: if no corpus or crawl document states a
  projected `/PCS`, the build closes a ledger row that no real file exercises — a ranking
  consideration under the campaign's current robustness-first weighting, and one the owner may
  overrule, as the curves answer just did.

## What the census is for — and what it is not

The census is **the build round's first commit, not a decision gate**: the tree's census-example
pattern, minutes rather than rounds, run once at the start. It is kept because it can delete the
expensive half of the scope — if the population carries `/WKT`, the registry is not needed at all
— and because it fixes the method set, the dialects, and the beneficiary count in one pass. It is
not a reason to hesitate, and this recommendation does not hide behind it.

**And it runs over the crawl, not the oracle's documents.** The wild-frequency questions here —
which of `/WKT` and `/EPSG` the world's files carry, which methods and codes they name, how many
documents benefit at all — are answered on `corpus-cache/safedocs/CC-MAIN-2021-31`, the 65 944
real web pages of the SafeDocs crawl, a Common Crawl sample unselected by any problem its
documents pose; `presentation_census` and its `xargs` over that crawl is the shape. The curated
corpora under `doc/corpora/` are reported beside it as the *control* — `doc/todo/01`'s own
instruction, read the control run as well as the crawl run — and are the wrong instrument on
their own for a wild-frequency question, being test suites and rule-breaking witnesses: files
known to pose problems, which is why nothing here counts them alone. What the control answers is
the other question: whether any test suite exercises the projection, which is regression value,
not frequency. The crawl's own limit is stated with it — a 2021 public-web snapshot — so a zero
there is strong evidence and not proof, which is why a zero routes to "coverage, taken on the
owner's answer here" rather than to "do not build".

## What the tree does meanwhile

The rows are `partial` with the reason named: the standard points at texts outside itself, and
the corpus population has not been counted.

## Recommendation

**Yes — build it, of the tree's own, with the census as the round's first commit.** Take the
`/WKT` fork as far as the files allow (parameters inline, no registry), the inverse methods the
census finds with each formula cited to Guidance Note 7-2's own section and its worked examples
as the test source, an accuracy budget documented as a choice, WKT1 carried only if the census
finds it, `/EPSG` codes answered from tables of just the codes documents use with the rest refused
by name — and if the census finds no documents at all, the round says so and the build is
coverage, taken on the owner's answer here rather than re-asked.