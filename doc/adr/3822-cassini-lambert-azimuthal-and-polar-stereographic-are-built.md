# 3822 — Cassini-Soldner, Lambert azimuthal equal area and the polar stereographics are built

Session 2280. Status: **accepted**. Context: ADR 3821, A1095, `doc/todo/79`; IOGP Guidance Note 7-2
(revised September 2019) sections 3.2.2, 3.3.2 and 3.3.1.2.

## Decision

Five methods, in `geospatial/projection/{cassini,azimuthal,polar}.rs`, both directions, each by its
section's formulas and each tested on its section's worked example through the EPSG code of the
system the example is stated for, so the table's units and the method are tested together:

- **Cassini-Soldner** (9806): section 3.2.2's series; Trinidad 1903 / Trinidad Grid (EPSG 30200),
  ellipsoid in Clarke's feet with a semi-minor axis, grid in Clarke's links.
- **Lambert Azimuthal Equal Area** (9820): the oblique aspect, and the polar aspect where the
  latitude of natural origin is ±90°, where the section says the oblique equations are indeterminate.
  `q` is section 3.1.3's `α`, already built for Albers. ETRS89 / ETRS-LAEA (EPSG 3035). The polar
  aspect has no printed example; its test is the reverse undoing the forward at both poles.
- **Polar Stereographic** variants A (9810), B (9829) and C (9830): one projection, three ways to
  state it. The radius from the pole is a constant times `t` in each (`2 a kO / k`, or `a mF / tF`),
  and variant C adds the false origin's radius `ρF`. UPS North (32661), Australian Antarctic (3032),
  Terre Adelie (2986; the example names Petrels 1972, whose conversion is the same).

## Choices

- **The polar longitude is `atan2` everywhere.** The section says λ = λO where E = FE. That holds on
  the origin's side of the pole and at the pole, and is wrong beyond it, where `atan2` gives the
  opposite meridian.
- **Pole cases the section rules out are refused**: variant A with a latitude of natural origin other
  than ±90° (note 1), and B or C with a standard parallel on the equator, which names no hemisphere.
- **Parameter codes.** 8832, latitude of standard parallel, is read into `parallel_1`, and 8833,
  longitude of origin, into the origin's longitude, so `Parameters` gains no field. The WKT reader
  takes the same codes and the five methods' names.

## Consequences

13 methods cover 5595 of the table's 5780 projected systems (96.8 %, from 95.2 %). The 34 left are
`doc/todo/79`'s.
