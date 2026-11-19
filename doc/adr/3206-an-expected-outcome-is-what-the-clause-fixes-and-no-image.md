# 3206 — An expected outcome is what the clause fixes, and never an image

Session 2126. Status: accepted. Context: RFC 0016 section 3; CLAUDE.md principle 5; trap 9.

## Decision

A portable test's expected outcome is stated in the manifest by kind:

- **pixels**: probe points classified as marked (every component at most 64) or unmarked (every
  component at least 191), at least five device pixels from any edge the clause fixes, black paint on
  white paper; an equality the clause states (`neutral`: three components within a tolerance); and a
  raster size where the page's size in pixels is a whole number;
- **text**: the exact Unicode string the clause's mapping gives, leading and trailing white space
  aside;
- **structure**: a one-line answer to a question the clause decides;
- **refusal**: what a malformed file must become where the clause says (§7.3.10's null, §7.3.7's
  absent entry), checked by the other three kinds.

No test carries a reference image, from this program or any other.

## Why

- A reference image encodes one processor's answers to everything the standard leaves open:
  anti-aliasing, how a fractional page becomes whole pixels, the level of a converted colour under
  §10.3. A comparison against it fails a processor for a choice the standard permits. pdf.js's
  reference tests are 1282 such comparisons against pdf.js's own earlier output, which is
  regression testing, not conformance.
- **The tolerance comes from the clause, so it is narrow where the clause is.** §10.4.2.2 states
  that a gray level "shall be equivalent to an RGB value with all three components the same", and
  §10.4.2.1 lets an ICC route replace the classic formula. So the test checks equality to 2 of 255 and
  says nothing about the level. A reference image would have fixed the level and failed every
  processor whose route differs.
- The thresholds are a quarter of the range from each end, because every pixel test asks marked or
  not. A processor whose black is not exactly 0 (an output profile's black point) still passes. One
  that draws the wrong shape still fails, because the probes sit where the clause, not a renderer,
  puts the edge.

## Cost

A probe tests a point, not a shape: a cap drawn as a slightly wrong curve between the probes passes.
More probes cost nothing to run, so each test places them where the clause's alternatives differ
(the round cap's corner, which a square cap would mark).
