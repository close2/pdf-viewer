# 1958 — A display system on another datum is reached through the parameters the file states

Session 1561, a clause round of batch eighty-nine. Status: **accepted** and **built**, on the
owner's answer `doc/questions/A436`. Builds the datum transformation that ADR 1593 refused and ADR
1918 section 3 left as §12.10.2's one unexecuted requirement. Context: ISO 32000-2 §12.10.2 (Table 269's `/DCS`); IOGP
Guidance Note 7-2 sections 4.1.1, 4.1.4, 4.2.3, 4.2.4 and 4.3.1; OGC 01-009 sections 7.3.18, 10.4
and 12.3.27; ISO 19162 sections C.3.3 and 20. Code: `crates/pdf-model/src/geospatial/datum.rs`
(`Helmert`, `transform`), `ellipsoid.rs` (`Ellipsoid::geocentric`, `Ellipsoid::geodetic`),
`system.rs` (`GeographicSystem::datum_shift`, the `TOWGS84` clause and the bound CRS), `mod.rs`
(`Geospatial::display`, `Refusal::DatumShift`, `Refusal::UnsupportedTransformation`); `examples/geospatial_census.rs` counts the clause. Tests: `geospatial::datum::tests`,
`geospatial::ellipsoid::tests::the_geocentric_conversion_reproduces_the_north_sea_example`,
`geospatial::system::tests::a_datums_towgs84_is_read_and_its_absence_is_none` and the three bound
CRS tests beside it,
`geospatial::tests::a_display_system_on_another_datum_is_reached_through_the_parameters_the_file_states`.
Row: §12.10.2 `partial` → `departed`, the one sentence departed from being ADR 1918's.

## 1. What the clause asks, and where its answer has to come from

Table 269's `/DCS` is "[a] projected or geographic coordinate system that shall be used for the
display of position values", and §12.10.2's prose illustrates it with exactly the case this tree
refused: a map "in a state planar coordinate system based on a 1927 datum" displayed "in the WGS84
datum corresponding to values reported by a GPS device". Reaching another datum is a transformation
whose parameters are in no table of ISO 32000-2 and in no datum's name — they are a survey's
measurement. A436 settles where they come from: **the file**, which states them one of two ways,
both read into one `DatumShift` — the datum it reaches and the Helmert transformation there:

- **the older form's `TOWGS84`**, in the datum of every census string's dialect, which OGC 01-009
  section 7.3.18 defines as up to seven Bursa Wolf parameters approximating the transformation
  from that datum to WGS 84, and ISO 19162 section C.3.3 documents inside the older `DATUM`. OGC
  01-009 is the text that defines the older form; it was fetched on 2026-10-15
  (`doc/third-party-data.md`) and is cited by section, never quoted;
- **ISO 19162 section 20's bound CRS**, the newer form's own statement: a source system, a target
  system and an abridged transformation between them, read where its method is one of the Helmert
  family — geocentric translations, Position Vector or Coordinate Frame, by EPSG code or by the
  Note's name — with section 20.2.3's units: metres, arc-seconds, and a scale relative to unity
  (1.0000035 for 3.5 ppm, so the scale difference is the number less one). Section 20.1 puts a
  transformation bound to a projected source on its base system, and it is kept there.

## 2. The method, every step the Guidance Note's

Guidance Note 7-2 section 4.3.1's table for the two-dimensional domain gives the Position Vector
transformation (geog2D domain) as five steps, and `datum::transform` is those steps in order: the
height set to zero (section 4.1.4 says the assumed height is usually zero and moves no horizontal
position while it is within kilometres of sea level), geographic to geocentric on the source
ellipsoid with the longitude first counted from Greenwich (section 4.1.1), the seven-parameter
Helmert transformation (section 4.2.3), geocentric to geographic on the target ellipsoid by section
4.1.1's non-iterative formula, and the height dropped. Each step is tested on the Note's worked
example: section 4.1.1's North Sea point in both directions, section 4.2.3's WGS 72 to WGS 84 in
both conventions, section 4.2.4's three-parameter translation and its exact reversal.

Each string states a transformation *out of* its own datum, so two systems meet on a datum both
reach — each its own by no parameters, and the one its statement names — in that datum's geocentric
coordinates: the source's parameters carry the point there and the target's carry it out again. Two
`TOWGS84` clauses meet in WGS 84; a bound CRS meets a display system on its target's datum.

## 3. What OGC 01-009 is read as saying

- **Position Vector, not Coordinate Frame.** OGC 01-009 section 10.4 writes the Bursa Wolf matrix
  with `-ez` in the first row and `+ez` in the second — Guidance Note 7-2 section 4.2.3's Position
  Vector matrix. The Note's warning that the two conventions differ only in the rotations' sign is
  why the reading is named; a test states the Note's Coordinate Frame example both ways, and ISO
  19162 section 20.3's EXAMPLE 3, a Coordinate Frame bound CRS, is the same Helmert as the
  `TOWGS84` of its rotations negated (`rPV = −rCF`). `Helmert` and `Bursa-Wolf` are aliases of
  both seven-parameter methods in ISO 19162 Table F.4, so a bound CRS naming its method by either
  alone names neither convention and is refused by that name.
- **Metres, arc-seconds, parts per million**, by section 12.3.27's attributes. Section 10.4 prints
  the scale as `S=(1+ppm*1000000)`; read by its own section 12.3.27 ("scaling in parts per
  million", paraphrased) and Guidance Note 7-2 Table 5, that is `M = 1 + ppm·10⁻⁶`.
- **Three, six or seven numbers.** Section 7.3.18 says only the first three or six are sometimes
  defined and the rest are then zero. Any other count is `Refusal::DatumShift`, raised only when a
  transformation asks for the clause, so a map with a malformed `TOWGS84` and no `/DCS` still reads.

## 4. Four choices, each because no text decides it

1. **The reverse step is the sign change.** Section 4.2.3 gives the exact reverse and says the EPSG
   Dataset documents the method's reversibility as the same formula with the parameters' signs
   changed, the exact reverse being a different method; the round trip so made misclosing by up to
   two centimetres in practical circumstances. The target's `TOWGS84` is applied backwards by that
   documented reversal. Cost: up to 2 cm, against ADR 1587's 0.0005″ (about 1.5 cm) budget for a
   projection — a datum transformation is itself an approximation (section 7.3.18 says so).
2. **WGS 84 is known by name, and by its figure.** Every `TOWGS84` is stated into WGS 84, so a
   display system on WGS 84 itself — the clause's own example — needs no parameters. It is
   recognised as the datum named `WGS_1984`, `WGS84` or `World Geodetic System 1984` (ESRI's `D_`
   off, case and separators ignored) **on** `a` = 6378137 m and `1/f` = 298.257223563 within 10⁻⁶,
   so a sphere named for WGS 84 is not taken for it. This is the one datum known by name, because it
   is the one the clause's parameters are defined against; no other name means anything.
3. **Meeting in geocentric coordinates, not in WGS 84 latitudes.** Two stated datums are joined
   without stopping on WGS 84's ellipsoid, so no height is dropped half-way; stopping there agrees
   within 0.0005″, which a test states.
4. **The same datum is decided as before** — the same name and the same ellipsoid, WGS 84's three
   spellings now one name by choice 2 — and needs no parameters even where both strings state them.

## 5. What is not done, and how each is said

No registry of parameters, no grid of shifts, no epoch: a datum that states no parameters, where the
other string's statement does not reach it, keeps `Refusal::DatumTransformation`, by both datums'
names, its sentence saying that the two strings state no transformation between them. A bound CRS
whose method reads a grid — section 20.3's own EXAMPLE 1, NADCON and its two files — or is outside
the Helmert family (Molodensky-Badekas, a time-dependent Helmert, NTv2) is refused by its method's
name (`Refusal::UnsupportedTransformation`), and a seven-parameter method missing one parameter by
that parameter's name. Each refusal of a statement is kept on the system and raised only when a
change of datum asks for it, so a map with an unreadable statement and no `/DCS` still reads.

## 6. The census, and why it is not this build's measure

`examples/geospatial_census.rs` now counts each `/GCS` and `/DCS` whose datum states `TOWGS84`,
calibrated on two scratch maps — NAD27 displayed in WGS 84 UTM zone 14N, one stating `TOWGS84` and
read as reached, one not and refused. No census string is the newer form (ADR 1586), so the bound
CRS has only the standard's own examples as witnesses. Over the crawl and the corpora it finds what
`doc/history/1561` records; no file states a `/DCS` at all (`doc/questions/Q436`), so nothing drawn
or said for any census document moves. A436 is the reason that does not decide the build: a corpus
cannot rank a requirement no document exercises.
