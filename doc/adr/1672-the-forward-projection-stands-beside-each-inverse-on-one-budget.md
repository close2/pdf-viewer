# 1672 — The forward projection stands beside each inverse, on one budget

Session 1418. Status: **accepted** and **built**. Builds on ADRs 1586 (the census and the fork),
1587 (the inverse and its budget) and 1593 (the affine reading between registration points);
decides nothing `doc/questions/Q271` asks, whose refusal stays by name. Supersedes nothing.
Context: ISO 32000-2 §12.10.2 (Table 269), §12.10.4; IOGP Guidance Note 7-2 (September 2019),
held under ADR 0187 and cited by section.
Code: `crates/pdf-model/src/geospatial/{projection,registration,mod,system}.rs`,
`crates/pdf-model/src/measurement.rs` (`Viewport::page_position`, `Geospatial::object_position`),
`crates/pdf-model/examples/geospatial_census.rs` (the round trip at the maps).

## 1. Why both directions

§12.10.4's projected system specifies the algorithms "used to transform points between geographic
coordinates and a two-dimensional (projected) coordinate system" — *between*, both ways. The
inverse serves a registration point and `/PCSM`'s position; the forward serves the two readings that
start from the earth: a projected `/DCS`, the system Table 269 says "shall be used for the display
of position values", and a position given to a viewport, which `/PCSM` can only place after the
position has been projected. Under either answer of Q271 the forward is also the leg a degree-shaped
registration would need to be fitted in the plane, which is why it is built before that answer.

## 2. What is built

- `Projection::forward` for all eight methods, by each section's forward formulas: Transverse
  Mercator (3.2.3.1, JHS), both Lambert conformal conics (3.1.1.1, 3.1.1.2), Mercator A and B
  (3.2.1), the Pseudo-Mercator (3.2.1.2), Albers (3.1.3) and the oblique stereographic (3.3.1.1).
  Each method's constants are one function both directions call, so the two cannot drift apart.
- `ProjectedSystem::grid`, the forward in the system's own unit; `Geospatial::display`, which now
  reaches a projected `/DCS` on `/GCS`'s datum as an easting and a northing in its stated unit.
  `display_position` keeps its degrees-only contract for a caller that writes degrees and says
  `DisplayIsProjected` where the display system is projected.
- `geospatial::AffineRegistration`, ADR 1593's least-squares fit, and its inverse; and
  `Viewport::page_position`, a position on the earth answered as the page point that reads it.

## 3. The budget, and an example that disagrees with itself

**The forward is held to ADR 1587's budget as a distance: 1.5 cm on the ground** of each worked
example's printed grid point, and the round trip to 0.0005″ at each example's point and around it.
One budget for both directions, because the half thousandth of a second is the claim; half of each
example's last printed digit is met by five of the seven examples, and the other two say why it
cannot be. **One does not follow from itself**: section 3.2.3.1's British National Grid example prints η = 0.0278542603 and
ξ = 0.8793956171, which this tree reproduces to every digit, and those give E = 577274.984 m and
N = 69740.492 m where the example prints 577274.99 and 69740.50. The test holds the example's
constants and intermediates to half their last digits and its printed grid point to the ground
budget. The Pseudo-Mercator's forward point is printed to nine places of a radian, about 3.5 mm of
northing, and lands 5.2 mm from its printed northing for that reason.

## 4. Where a forward formula has no answer

A non-finite grid point is never returned, and four domains are stated rather than left to `f64`:
Mercator's poles and a cone's far pole are at infinity, refused within 1e-12 rad (where `tan` of
the nearest double to π/2 is a finite number and would otherwise pass); the stereographic's
antipode, where its denominator is zero; a Transverse Mercator point more than 90° from the
central meridian, where `cos β sin(λ − λO)` is the mirror point's and the formulas' `asin` would
answer for that point instead; and the Pseudo-Mercator poleward of 88°, where section 3.2.1.2 says
its formula fails and is not to be used — applied to the inverse's latitude too, the one change to
a built inverse. Section 1.4's wrap is applied to λ − λO on the way in.

## 5. The inverse of ADR 1593's map is the same map run backwards

`page_position` inverts the fit rather than fitting the other direction, because two least-squares
fits disagree wherever the points lie on no affine map, and a position given back would then not
land on the point that read it. Through `/PCSM` it projects and solves the matrix's first two rows
with `z` zero, as a page's position has it, and refuses a singular matrix. A position the map
places outside `/Bounds` is refused: the table's area "for which geospatial transformations are
valid" bounds both directions.

## 6. At the census's maps

The worked examples are seven points; the maps are where a person points. The census
(`examples/geospatial_census.rs`, 90 763 files behind the lock, 137 s, 8.81 GiB peak) now carries
every projected `/GCS` whose points are shaped as degrees forward and back at each of its
registration points, read as the base system's degrees — the instrument's reading, not the
program's, which keeps refusing them under Q271. **Every one closes within 0.0005″ and none is
refused**: 84 Transverse Mercator systems, 50 Lambert 2SP, 10 Lambert 1SP, 6 Mercator variant B,
4 Albers, 3 oblique stereographic and 3 Pseudo-Mercator; no census map is Mercator variant A, which
the Makassar example alone holds. ADR 1586's population is unchanged: 158 projected maps, every one degree-shaped, none with `/PCSM`. One map per method is a
unit test (`every_method_closes_at_a_census_maps_registration_point`), its WKT as the crawl file
states it.

## 7. What it leaves

`viewer_core::located::fitted` is a second copy of the fit until slot 2's crate takes the one-hunk
patch that makes it call `AffineRegistration` (the round's report carries it), and the windows
write a projected `/DCS`'s refusal sentence until a host writes eastings; both are the hosts'.
The 158 degree-shaped maps stay refused until Q271 is answered.
