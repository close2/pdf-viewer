# 79 — The projection methods past the thirteen, in order

Status: **scheduled** — the owner's answer `doc/questions/A1095` (2026-11-27): carry the EPSG registry
and "[b]uild the remaining projection methods beyond the initial eight [...] as their own follow-up
rows". The ledger holds one row per numbered subclause (`doc/PLAN.md` §5a), so each method is an entry
of this list, and §12.10.4's row stays `partial` naming it until the last is built (ADR 3823).
Instrument: the count below is `crates/pdf-model/src/geospatial/epsg/dataset.rs`'s own — each method
code against the projected systems whose conversion names it, deprecated codes included — and the
command that prints it is in ADR 3823. The method sections are IOGP Guidance Note 7-2's
(`doc/md/IOGP-373-07-02.md`), revised September 2019.

## What is built

Forty-six method codes, 5767 of the table's 5780 projected systems (99.8 %), four of them refused by
the parameters the table cannot carry (below): the census's eight (ADRs 1586,
1587, 1672) — Transverse Mercator, Lambert Conic Conformal 1SP and 2SP, Mercator A and B,
Pseudo-Mercator, Albers, oblique stereographic — and ADR 3822's five, each tested on its section's
worked example through the EPSG code the example is stated for: Cassini-Soldner (3.2.2, Trinidad
Grid, EPSG 30200), Lambert Azimuthal Equal Area (3.3.2, ETRS-LAEA, EPSG 3035), Polar Stereographic
A, B and C (3.3.1.2, UPS North 32661, Australian Antarctic 3032, Terre Adelie 2986).

And this list's first five (ADRs 3870, 3871): Colombia Urban (3.4.3, Bogota urban grid, EPSG 6247),
Transverse Mercator South Orientated (3.2.3.3, Hartebeesthoek94 / Lo29, EPSG 2053), Hotine Oblique
Mercator variant B (3.2.4.1, Timbalai 1948 / R.S.O. Borneo (m), EPSG 29873) and variant A (the same
example with its false coordinates at the natural origin, as the section states it, built from the
example's parameters because no carried system states them with variant A), and Lambert Conic
Conformal West Orientated (3.1.1.3, which prints no example: tested on Qornoq 1927 / Greenland zone
1 east, EPSG 2297, against the 1SP formulas the section states it as). Hotine's special case at an
azimuth of 90° is read as a choice (ADR 3871, tested on CH1903 / LV03, EPSG 21781).

And rows 7 to 11 (ADRs 3875, 3876): Lambert Azimuthal Equal Area (Spherical) (3.3.2.1, which prints
no formulas and names USGS Professional Paper 1395: its formulas and Appendix A example, R = 3, and
NSIDC EASE-Grid North, EPSG 3408 and 3973), American Polyconic (3.1.5, which prints no example: the
same paper's ellipsoid example, and SIRGAS 2000 / Brazil Polyconic, EPSG 5880, by round trip),
Lambert Conic Conformal 2SP Michigan (3.1.1.5, NAD27 / Michigan Central, EPSG 6201), and Krovak in
its four forms (3.1.2.1 to 3.1.2.4, S-JTSK (Ferro) / Krovak, EPSG 2065, and its East North form,
5221). The modified forms are tested on their section's printed parameters and through their
systems (5224, 5225 and the Greenwich forms 5515 and 5516), whose evaluation point and C1 to C10
the generator reads from IOGP's registry, PROJ's SQL holding seven parameters a conversion (ADR
3897).

And rows 12 to 16 and 18 (ADRs 3877 to 3880): Equal Earth (3.4.4, WGS 84 / Equal Earth Americas,
EPSG 8858), Lambert Cylindrical Equal Area and its spherical form (3.2.6, which prints no formulas and
names Professional Paper 1395: its two normal-aspect examples, and EPSG 6933, 3410 and 3975),
Equidistant Cylindrical and its spherical form (3.2.5, WGS 84 / World Equidistant Cylindrical, EPSG
4087; 3.2.5.1 prints no example and is tested on its formulas through 4088, 3786 and 32662), with the
two codes those replaced, 9842 and 9823, read as their latitude of natural origin being the parallel
(ADR 3880), Bonne in both orientations (3.1.4 and 3.1.4.1, which print no example: the paper's
ellipsoid example, and EPSG 2963 and 5017), Laborde Oblique Mercator (3.2.4.2, Tananarive (Paris) /
Laborde Grid, EPSG 29701, and its Greenwich form 8441; four printed lines read in ADR 3878), and the
Transverse Mercator Zoned Grid System (3.2.3.2, which prints no example: WGS 84 / UTM grid system,
EPSG 32600 and 32700, against each zone's own system; ADR 3879).

And rows 19 to 27 (ADRs 3885 to 3888), each the method of one system, each tested on its worked example
through that system: Hyperbolic Cassini-Soldner (3.2.2.1, Vanua Levu Grid, EPSG 3139, in links; its
first estimate's divisor is a length in chains and the example's `ϕ1'` drops its sign), Modified
Azimuthal Equidistant (3.3.3.1, Guam 1963 / Yap Islands, 3295; its distance `s` is evaluated as the
sine rule's quotient with `sin α` divided out, which is also its `sin α = 0` case), the Guam
Projection (3.3.3.2, Guam SPCS, 3993, its three reverse steps), Mercator (Spherical) and the code it
replaced, Mercator (1SP) (Spherical) (3.2.1.1's example on its sphere, and EPSG 3785 against 3857;
ADR 3886), Lambert Conic Conformal (2SP Belgium) (3.1.1.4, BD72 / Belge Lambert 72, 31300), Lambert
Conic Near-Conformal (3.1.1.6, Deir ez Zor / Levant Zone, 22700, its reverse by the section's
iteration and `θ′` signed as `r′` is), the New Zealand Map Grid (3.4.1 prints no formulas: the Office
of the Surveyor-General's Technical Report 4.2 and its three test points, through 27200; ADR 3887),
the Tunisia Mining Grid (3.4.2, EPSG 22300, and the decree's tabulated ends), and Transverse Mercator
3D (no section: the Dataset's own description and example, LUREF / Luxembourg TM (3D), 9895; ADR
3888).

**What is left of the table** is the three methods below, 13 systems, and all three wait on Q1114.

Rows 6, 10 and 17, Azimuthal Equidistant (1125), Lambert Conic Conformal 1SP variant B (1102) and
Local Orthographic (1130), are not in the September 2019 revision (its 3.3.5 is the Orthographic,
9840). The revision that has them sits behind a download form asking for personal details
(`doc/questions/Q1114`), and all three stay refused by name.

## The order

By how many projected systems state each, then by the section's length. Every one is built the same
way: its section's formulas in both directions in `geospatial/projection/`, its EPSG method code in
`geospatial/epsg/mod.rs`'s `method` and `system.rs`'s `named`, its parameter codes where they are new,
and its worked example as the test's expected value, through the code of the example's system where
the table holds it. Each built moves §12.10.4's note; the last moves the row to `implemented`.

| # | method | EPSG | systems | GN 7-2 section |
|---|---|---|---|---|
| 1 | Colombia Urban, built | 1052 | 32 | 3.4.3 |
| 2 | Transverse Mercator (South Orientated), built | 9808 | 28 | 3.2.3.3 |
| 3 | Hotine Oblique Mercator (variant A), built | 9812 | 23 | 3.2.4.1 |
| 4 | Hotine Oblique Mercator (variant B), built | 9815 | 19 | 3.2.4.1 |
| 5 | Lambert Conic Conformal (West Orientated), built | 9826 | 19 | 3.1.1.3 |
| 6 | Azimuthal Equidistant, waits on Q1114 | 1125 | 7 | 3.3.3 |
| 7 | Lambert Azimuthal Equal Area (Spherical), built | 1027 | 6 | 3.3.2.1 |
| 8 | American Polyconic, built | 9818 | 5 | 3.1.5 |
| 9 | Lambert Conic Conformal (2SP Michigan), built | 1051 | 4 | 3.1.1.5 |
| 10 | Lambert Conic Conformal (1SP variant B), waits on Q1114 | 1102 | 4 | 3.1.1.2 |
| 11 | Krovak, North Orientated, Modified, Modified (North Orientated), built | 9819, 1041, 1042, 1043 | 10 | 3.1.2.1 to 3.1.2.4 |
| 12 | Equal Earth, built | 1078 | 3 | 3.4.4 |
| 13 | Lambert Cylindrical Equal Area, and its spherical form, built | 9835, 9834 | 3 | 3.2.6, 3.2.6.1 |
| 14 | Equidistant Cylindrical, and its spherical forms, built | 1028, 1029, 9823, 9842 | 5 | 3.2.5, 3.2.5.1 |
| 15 | Bonne (South Orientated), built (and Bonne, 9827) | 9828 | 2 | 3.1.4.1 |
| 16 | Laborde Oblique Mercator, built | 9813 | 2 | 3.2.4.2 |
| 17 | Local Orthographic, waits on Q1114 | 1130 | 2 | not in the September 2019 revision |
| 18 | Transverse Mercator Zoned Grid System, built | 9824 | 2 | 3.2.3.2 |
| 19 | Hyperbolic Cassini-Soldner, built | 9833 | 1 | 3.2.2.1 |
| 20 | Modified Azimuthal Equidistant, built | 9832 | 1 | 3.3.3.1 |
| 21 | Guam Projection, built | 9831 | 1 | 3.3.3.2 |
| 22 | Mercator (1SP) (Spherical), built (and Mercator (Spherical), 1026) | 9841 | 1 | 3.2.1.1 |
| 23 | Lambert Conic Conformal (2SP Belgium), built | 9803 | 1 | 3.1.1.4 |
| 24 | Lambert Conic Near-Conformal, built | 9817 | 1 | 3.1.1.6 |
| 25 | New Zealand Map Grid, built | 9811 | 1 | 3.4.1 |
| 26 | Tunisia Mining Grid, built | 9816 | 1 | 3.4.2 |
| 27 | Transverse Mercator 3D, built | 1111 | 1 | not a section of the September 2019 revision; the Dataset's own description is read first |

The sections are the Guidance Note's table of contents; whether each prints a worked example, and
for which system, is read by the round that takes the row, and that example (or, where none is
printed, a statement that none is and what stands in its place) is the row's test.

**Not in this list**: a method a WKT string names that no projected system of the carried table
uses — ESRI's own names past the ones `system.rs` maps, and the vertical perspective of section
3.3.4.2, which the crawl census found in a WKT string. Each is refused by its name as it is now, and
is taken when a document or an owner's word asks for it.
