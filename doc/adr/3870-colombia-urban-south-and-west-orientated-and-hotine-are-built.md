# 3870 — Colombia Urban, the south and west orientated projections and Hotine are built

Session 2292. Status: **accepted**. Context: A1095, ADRs 3822 and 3823, `doc/todo/79` rows 1 to 5;
IOGP Guidance Note 7-2 (revised September 2019) sections 3.4.3, 3.2.3.3, 3.2.4.1 and 3.1.1.3.

## Decision

Five methods, both directions, each by its section's formulas and tested on its section's worked
example through the EPSG code of the system it is stated for:

- **Colombia Urban** (1052), `projection/urban.rs`: section 3.4.3's closed forms, which need the
  height of the projection origin (parameter 1039, `Parameters::origin_height`). MAGNA-SIRGAS /
  Bogota urban grid (EPSG 6247) to the printed millimetre.
- **Transverse Mercator (South Orientated)** (9808): section 3.2.3.3 states it as the general case's
  terms subtracted from the false ones, so it is the JHS evaluation of 3.2.3.1 with both offsets
  negated, both directions. Hartebeesthoek94 / Lo29 (EPSG 2053), to ADR 1672's 1.5 cm since its
  point is printed to a thousandth of a second.
- **Hotine Oblique Mercator, variants A and B** (9812, 9815), `projection/oblique.rs`: section
  3.2.4.1's constants, forward and reverse, with the azimuth of the initial line (8813) and the
  angle from the rectified to the skew grid (8814) as new parameters and the projection centre
  (8811, 8812), the scale on the initial line (8815) and the easting and northing at the centre
  (8816, 8817) in the roles their natural-origin counterparts play. Variant B on Timbalai 1948 /
  R.S.O. Borneo (m) (EPSG 29873); variant A on the same example with its false coordinates at the
  natural origin, which the section states, built from its parameters since no carried system
  states them with variant A. The 90° azimuth is ADR 3871's.
- **Lambert Conic Conformal (West Orientated)** (9826): section 3.1.1.3's `W = FE − r sin θ`, the
  1SP cone with its easting mirrored. The section prints no example; Qornoq 1927 / Greenland zone 1
  east (EPSG 2297) is checked against a 1SP projection of the same parameters — the westing is
  2·FE less the 1SP easting, the northing the same — and returns from its own forward.

`system.rs` reads each by its method code and by its Dataset name in a WKT string. The older form's
and ESRI's oblique Mercator names (`Hotine_Oblique_Mercator`, `…_Azimuth_Natural_Origin`) do not say
which variant or which rectified angle they mean, and stay refused by name.

## Consequences

§12.10.4 stays `partial`: 64 of the table's 5780 projected systems over 29 methods remain, the
count ADR 3823's command prints, from `doc/todo/79`'s sixth row on.
