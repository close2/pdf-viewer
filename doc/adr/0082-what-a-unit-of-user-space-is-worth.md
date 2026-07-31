# ADR 0082 — What a unit of user space is worth

Status: accepted, 2026-07-31.

## Context

§12.9 and §12.10 were nine `silent` rows and one sentence between them: a viewport is a rectangle
of a page with its own idea of what a user-space unit means, and a measure dictionary is what it
means — a scale for a CAD drawing (§12.9), or the earth for a map (§12.10).

Nothing in either clause puts a mark on a page. That is the clause's own position rather than an
excuse: a measure dictionary "shall provide information for formatting the resulting values into
textual form for presentation in a graphical user interface".

## Decision

`pdf-model/src/measurement.rs` reads Tables 265 through 272 — viewports, rectilinear and
geospatial measures, number formats, coordinate systems, point data — and implements the one
thing either clause states as an algorithm.

Three findings made it worth doing as a clause reading rather than as a data dump.

**The selection rule runs backwards.** Viewports overlap, and §12.9.1 does not say the smallest or
the innermost wins:

> The dictionaries in the VP array shall be in drawing order. Since viewports might overlap, to
> determine the viewport to use for any point on a page, the dictionaries in the array shall be
> examined, starting with the last one and iterating in reverse, and the first one whose BBox
> entry contains the point shall be chosen.

Last drawn wins. A reader that searched forwards would answer with the background plan's scale
inside every detail inset, and would look right on every page with one viewport.

**A rectangle whose corner order is meaning, not noise.** Every other rectangle in this crate is
normalised on read. Table 265's `/BBox` is not, because the clause makes the ordering carry
information: it "shall determine the orientation of the measuring coordinate system (that is, the
direction of the positive x and y axes) in this viewport, which may have a different rotation
from the page". `Viewport::contains` normalises for the containment test alone.

**§12.9.2 is an algorithm with a worked example, so the example is the test.** A number format
array is a sequence of units in descending granularity, and the algorithm carries each unit's
*fractional remainder* into the next. The clause's own example — 1.4505 miles through miles, feet
and inches with `/F /F /D 8` — is `1 mi 2,378 ft 7 5/8 in`, and every part of the algorithm is
load-bearing in that one string: two carries, the `/RT` comma in 2,378, and 0.68 of an inch
rounded to the nearest eighth. No corpus document exercises any of it (trap 8), which is exactly
why the clause's own example matters.

## Where the boundary is, and why it is not this program's to move

§12.10's transformation — a point on a page to a latitude — is stated by reference to two things
outside ISO 32000-2: an EPSG reference code, whose database is "administered by the International
Association of Oil and Gas Producers (OGP)", and a Well Known Text string, whose format "is
specified in ISO 19162". Implementing that means a geodesy library and a registry, and *guessing*
at it produces coordinates that look right and are somewhere else.

What is usable without any of it is what the file states directly: `Geospatial::registration`
pairs `/GPTS` with `/LPTS` — the same points in two coordinate systems — and
`Geospatial::matrix_has_priority` answers Table 269's own precedence, which needs two entries at
once: `/PCSM` "has priority over GPTS" where present, and "should be ignored" when `/GCS` is
geographic.

## The one witness, and what is wrong with it

One document of the 974 states a `/VP`: `bug1146106.pdf`, whose viewport is `GEO`, whose `/GCS` is
a geographic system given as 145 characters of WKT, and which registers four corners. Two things
about it are the *file* being wrong, and both are asserted rather than accommodated:

- Its `/Name` begins with a UTF-16 **little**-endian byte order mark. §7.9.2.2 defines three
  encodings — UTF-16BE behind `FEFF`, UTF-8 behind `EFBBBF`, and otherwise PDFDocEncoding — so by
  the clause's own elimination this name *is* PDFDocEncoding, and it decodes to the producer's
  bytes rather than to `Layers`. Reading it as UTF-16LE would mean implementing an encoding the
  standard does not define on a guess about which program wrote the file.
- Its `/BBox` is stated upper-left first, which Table 265 forbids ("in normalised form; that is,
  lower-left followed by upper-right"). Kept as written, because of the orientation sentence
  above: this viewport's measuring y axis runs down the page.

## Consequences

- `silent` falls 86 → **77**, the largest fall of any session: §12.9, §12.9.1, §12.10, §12.10.1
  and §12.10.2 become `partial`, and §12.9.2, §12.10.3, §12.10.4 and §12.10.5 `implemented`.
- Clause 12's silences fall to 66, and what is left of them is forms, signatures and collections.
- No gate moves, and none could.
- The two `partial` rows owe the same thing and it is not a clause: a measuring tool, which is a
  person dragging between two points.
