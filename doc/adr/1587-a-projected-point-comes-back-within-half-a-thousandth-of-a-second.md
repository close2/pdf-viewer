# 1587 — A projected point comes back within half a thousandth of a second

Status: accepted. Session 1375. Builds on ADR 1586 (the census and the fork) and ADR 1191 (§12.10
read as data); carries out `doc/questions/A171`'s obligation that the accuracy budget be
documented as a choice. Supersedes nothing.
Context: ISO 32000-2 §12.10.2–§12.10.4; IOGP Guidance Note 7-2 (Revised September 2019) and ISO
19162 (OGC 18-010r11), both held under ADR 0187 (`doc/third-party-data.md`).
Code: `crates/pdf-model/src/geospatial/{mod,wkt,system,projection,ellipsoid}.rs`.

## 1. The budget

**The inverse returns a latitude and a longitude within 0.0005″ — half a thousandth of an
arc-second, about 1.5 cm on the ground — of the method's own definition**, over the region the
method is used for. It is a choice, because there is no oracle: no renderer this tree compares
with implements a projection, and a pixel gate cannot see one. The number is chosen for three
reasons. Guidance Note 7-2 prints its worked examples' latitudes to 0.001″, so half the last
printed digit is the finest claim its examples can test. Section 3.2.3.1 puts the difference
between its two Transverse Mercator formulas at no more than 0.0005″ within 4° of the central
meridian, so the budget is the size of the disagreement between the Note's own two answers. And
a `/GPTS` point is stated to the centimetre at best, so nothing finer is a coordinate any file
carries.

**The tests are the Note's worked examples, each to half its last printed digit**: Transverse
Mercator (British National Grid, section 3.2.3.1, JHS formulas), Lambert 2SP (Texas South Central
in US survey feet, 3.1.1.1), Lambert 1SP (Jamaica, 3.1.1.2, printed to 0.01″), Mercator variants A
and B (3.2.1), Pseudo-Mercator (3.2.1.2) and the oblique stereographic (RD New, 3.3.1.1). Albers
(3.1.3) prints no example — the Note points elsewhere for one — so its inverse is held to the
section's own forward formulas at ISO 32000-2 §12.10.4's EXAMPLE 2 parameters. The southern
Lambert cone is held the same way (section 3 below).

## 2. What the arithmetic does at the edges

- **Iteration.** Each iterated latitude stops when a step changes it by less than 1e-14 rad and is
  refused after 32 steps; the Note expects three or four.
- **Series.** The Mercator, Albers and Transverse Mercator inverses are the Note's truncated series
  (to e⁸, e⁶ and n⁴); their truncation error is inside the budget where the methods are used, and
  the Transverse Mercator's JHS formulas are the Note's recommendation for up to 40° from the
  central meridian.
- **Domain.** A non-finite answer, or a latitude past a pole, is a refusal naming the method —
  never a number.

## 3. Five readings the texts leave open, each a choice

1. **The older WKT form's parameter units.** ISO 19162 section C.3.4 says they are ambiguous. An
   angle is read in the base system's angular unit and a length in the projected system's linear
   unit, because every census State Plane system states its false easting in US survey feet beside
   `UNIT["Foot_US",…]` and the false origin lands where the system's name puts it only that way.
   The newer form states units per parameter; omitted, ISO 19162 section 9.3.4 makes them metres
   and degrees.
2. **A family name.** `Lambert_Conformal_Conic` is the 2SP method with two distinct parallels and
   the 1SP method with one (its origin on that parallel); `Mercator` is variant B with a standard
   parallel and variant A with a scale factor. Each method's definition in the Note decides it; a
   combination neither defines is refused.
3. **ESRI's `Mercator_Auxiliary_Sphere`, type 0**, is the Pseudo-Mercator: two of the three files
   that name it state `/EPSG 3857` in the same dictionary, and the Note's own example of section 3.2.1.2 is
   EPSG 3857. Any other type, or a parallel off the equator, is refused.
4. **An absent false easting or northing is zero**: every method subtracts it, and no offset is
   none. No other parameter defaults; a missing one is refused by its Note name, and a parameter
   whose name this reader does not know is refused rather than dropped.
5. **A missing comma between two tokens** is accepted, because §12.10.4's own EXAMPLE 2 is
   written without one; nothing else outside ISO 19162 section 6 is.

And one sign the Note leaves implicit: section 3.1.1.1 gives r′ the sign of n, and the same sign
is applied to θ′'s arguments — the forward formulas make both of them r times a trigonometric
function — which leaves the northern cone and the Note's example unchanged and is what puts a
southern cone's point back where its own forward formulas put it. Albers takes it for the same
reason. The oblique stereographic centred south of the equator is refused: the Note's sentence
about reversing signs there is not precise enough to evaluate without an example.

## 4. What this decides for later rounds

A new method is added by its section of the Note, with that section's worked example as its test
to this budget. Hotine's oblique Mercator (4 census documents) is the next by count and is not
built: ESRI's spelling states no angle from the rectified to the skew grid, which the Note's
method needs, and supplying it would be a convention no text this tree holds states.
