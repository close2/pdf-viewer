# 1678 — A window writes a projected display system's easting and northing

Session 1421. Status: **accepted** and **built**. Builds on ADRs 1593 (the position between
registration points), 1672 (the forward projection and `Geospatial::display`) and 1191 (the
measuring gesture). Supersedes nothing; decides nothing `doc/questions/Q271` asks.
Context: ISO 32000-2 §12.10.2, Table 269 (`/DCS`); IOGP Guidance Note 7-2, section 3.2.3.1.
Code: `crates/viewer-core/src/located.rs` (`Located::At::display`), `crates/viewer-host/src/measuring.rs`
(`grid`, `geospatial_sentence`), `crates/viewer-confined/src/protocol.rs` (`encode_located`,
`decode_located`, greeting `PDFVCF10`). Tests: `viewer-core`'s
`a_projected_display_system_is_answered_as_its_easting_and_northing`, `viewer-host`'s
`a_display_system_writes_its_own_coordinates`, `viewer-confined`'s
`a_located_point_crosses_in_every_display_shape`; `tools/drive-windows.sh` step 58.

## 1. What changed

Table 269 makes `/DCS` "[a] projected or geographic coordinate system that shall be used for the
display of position values". `viewer_core::located` read it through `display_position`, whose
contract is degrees, so a projected `/DCS` reached every window as `DisplayIsProjected`'s
sentence. It now reads it through `Geospatial::display`, and `Located::At::display` carries
`pdf_model::geospatial::Displayed` itself: a latitude and a longitude, or an easting and a northing
with the unit the system's string names. No message was added; the variant changed shape, so the
confined wire's `display` byte gained a fourth value (3: two `f64`s and the unit) and the greeting
moved.

## 2. The wording, which is a choice

Table 269 leaves the form to the processor ("[f]ormatting the displayed representation of these
values is controlled by the interactive PDF processor"). A grid position is written
`easting 500000.00 Meter, northing 5260729.73 Meter`: each axis **named**, because a letter `E`
after a number is also a hemisphere and a projected system's axes are not a geographic one's;
**two places** of the system's own unit, because the forward is held to 1.5 cm on the ground
(ADR 1672) and a third place of a metre would print a digit the reading does not hold; the unit **as
the system's string spells it**, because the string is the file's and the program has no table of
unit names to translate it by.

## 3. The expected values

The `viewer-core` test's map is a north-up rectangle of degrees on ETRS89 with a UTM zone 32N
`/DCS`; its middle is 47.5° N, 9° E, on the zone's central meridian, where section 3.2.3.1's
formulas reduce to the false easting and the scale factor times the meridian arc —
0.9996 × 5 262 834.867 m on GRS 1980, the arc integrated numerically from the ellipsoid's meridional
radius of curvature. That is a value derived from the method, not from the program under test.
