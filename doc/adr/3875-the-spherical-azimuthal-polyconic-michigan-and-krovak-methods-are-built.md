# 3875 — The spherical azimuthal, Polyconic, Michigan and Krovak methods are built

Session 2293. Status: **accepted**. Context: A1095, ADRs 3823 and 3870, `doc/todo/79` rows 6 to 11;
IOGP Guidance Note 7-2 (revised September 2019) sections 3.3.2.1, 3.1.5, 3.1.1.5 and 3.1.2.1 to
3.1.2.4; USGS Professional Paper 1395 (Snyder 1987, public domain).

## Decision

Seven method codes, both directions, each tested on a printed worked example:

- **Lambert Azimuthal Equal Area (Spherical)** (1027), `projection/spherical.rs`. Section 3.3.2.1
  prints no formulas. It says "See USGS Professional Paper 1395 [...] for formulas and example", so
  the formulas are the paper's (24-2, 22-4, 22-5; 20-14, 20-15). `R` is the sphere's own axis, or
  the authalic radius `RA` of the implementation notes for an ellipsoid, as the section says. The
  test is the paper's Appendix A example (R = 3) to its seven decimals. It also checks the polar
  aspect's equator through NSIDC EASE-Grid North (EPSG 3408) and its WGS 84 form (3973).
- **American Polyconic** (9818), `projection/polyconic.rs`. Section 3.1.5 prints no example (it
  names GIGS 5107, which is not held). The test is the paper's Appendix A ellipsoid example, the one
  the section's formulas come from, to its decimetre. SIRGAS 2000 / Brazil Polyconic (5880) is
  tested by a round trip. ADR 3876 records two readings of the reverse.
- **Lambert Conic Conformal (2SP Michigan)** (1051): the 2SP formulas with `a K` for `a`, the
  ellipsoid scaling factor being parameter 1038 (`Parameters::ellipsoid_scale`). NAD27 / Michigan
  Central (EPSG 6201) in US survey feet, to the printed hundredth.
- **Krovak, North Orientated, Modified, Modified North Orientated** (9819, 1041, 1042, 1043),
  `projection/krovak.rs`. Parameter 1036 (co-latitude of the cone axis) takes Hotine's αC role,
  8818 the one parallel and 8819 the scale. A grid point is carried westing first, and the north
  forms negate both coordinates after the false ones are added. Tested on S-JTSK (Ferro) / Krovak
  (2065) and its East North form (5221), longitudes from Ferro as the base system counts them.
  The Modified forms take section 3.1.2.3's evaluation point (8617, 8618) and C1 to C10 (1026 to
  1035).

## What is not built, and why

- **Krovak Modified through its codes**: the carried table is written from PROJ's SQL, whose
  conversion records hold seven parameters, and the polynomial is twelve more. Systems 5224, 5225
  and the two Greenwich forms are refused by name ("ordinate 1 of evaluation point"). The method is
  tested on the section's printed parameters.
- **Azimuthal Equidistant (1125)** and **Lambert Conic Conformal (1SP variant B) (1102)**: the
  September 2019 revision has no section for either. Its 3.3.3 is a heading over 3.3.3.1, and 1102
  is not mentioned. The revision that has them (v74, July 2026) is behind a download form asking for
  personal details, and that is the owner's to give (`doc/questions/Q1114`). Professional Paper
  1395's ellipsoidal azimuthal equidistant is the Guam and Micronesia forms (9831, 9832), not 1125.

Covered: 5741 of 5780 projected systems by method code, 4 of them refused by the parameter the
table cannot carry; 39 left over 22 methods (ADR 3823's command).
