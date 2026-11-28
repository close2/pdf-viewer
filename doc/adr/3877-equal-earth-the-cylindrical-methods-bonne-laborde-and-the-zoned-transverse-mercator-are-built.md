# 3877 — Equal Earth, the cylindrical methods, Bonne, Laborde and the zoned Transverse Mercator are built

Session 2294. Status: **accepted**. Context: A1095, ADRs 3823 and 3875, `doc/todo/79` rows 12 to 18;
IOGP Guidance Note 7-2 (revised September 2019) sections 3.4.4, 3.2.6, 3.2.6.1, 3.2.5, 3.2.5.1, 3.1.4,
3.1.4.1, 3.2.4.2 and 3.2.3.2; USGS Professional Paper 1395 (Snyder 1987, public domain), fetched by
round 2293.

## Decision

Eleven method codes, both directions, in `geospatial/projection/`:

- **Equal Earth** (1078), `equal_earth.rs`. Tested on section 3.4.4's example through WGS 84 / Equal
  Earth Americas (EPSG 8858), to the printed millimetre, and the three carried systems by round trip.
  The printed denominator is checked as the derivative of the northing's polynomial (0.243318 = 3 ×
  0.081106, 0.006251 = 7 × 0.000893, 0.034164 = 9 × 0.003796). The spherical form is the same
  evaluation with `qP = 2`.
- **Lambert Cylindrical Equal Area** (9835) and **its spherical form** (9834), `cylindrical.rs`.
  Section 3.2.6 prints no formulas and names Professional Paper 1395 "for formulas and example": its
  equations 10-13 to 10-15 and 3-12, back by 10-26, 10-27 and series 3-18 (which the paper gives in
  place of 10-17 and 3-16). Tested on the paper's two normal-aspect examples (Clarke 1866, 5°N 78°W;
  R = 1, 35°N 80°E), and EPSG 6933, 3410 and 3975 by round trip and by the pole's northing.
- **Equidistant Cylindrical** (1028, and 9842) and **its spherical form** (1029, and 9823). Section
  3.2.5's example through WGS 84 / World Equidistant Cylindrical (EPSG 4087), to the printed
  centimetre. Every printed fraction of the arc series to `e¹⁴` was checked against an exact
  expansion of the section's own equation (1), and the reverse series in `n` by a round trip at
  every tenth of a degree to 1e-11°. Section 3.2.5.1 prints no example; it is tested on its
  formulas through EPSG 4088, 3786 and 32662. ADR 3880 has the readings.
- **Bonne** (9827) and **Bonne (South Orientated)** (9828), `bonne.rs`. Neither section prints an
  example. Tested on Professional Paper 1395's ellipsoid example (19-8 to 19-14), whose symbols the
  section uses, with the origin mirrored south for the sign of ϕO, and EPSG 2963 and 5017 by round
  trip. The pole's "use λ = λO" is tested at the pole and a hair off it. A latitude of natural origin
  on the equator divides by zero and is refused by name.
- **Laborde Oblique Mercator** (9813), `laborde.rs`. Section 3.2.4.2's example through Tananarive
  (Paris) / Laborde Grid (EPSG 29701) and its Greenwich form (8441), to the millimetre, and the
  section's comparison table. ADR 3878 has four readings.
- **Transverse Mercator Zoned Grid System** (9824), `zoned.rs`. No example is printed; WGS 84 / UTM
  grid system (EPSG 32600, 32700) is tested as each zone's own UTM system with the zone prefixed.
  ADR 3879 has the readings.

Parameters 8830 (initial longitude) and 8831 (zone width) are new, in the table and in a WKT string.

## What is not built, and why

**Local Orthographic (1130)**: the September 2019 revision has no section for it (3.3.5 is the
Orthographic, 9840). The Dataset's own record for 1130 says only "See EPSG Guidance Note 7-2". It
waits with 1125 and 1102 on the revision behind the download form (`doc/questions/Q1114`); that
question's answer decides it too.

Covered: 5758 of 5780 projected systems by method code (ADR 3823's command), 4 of them refused by
Krovak Modified's polynomial; 22 left over 12 methods.
