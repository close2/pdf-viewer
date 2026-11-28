# 3888 — Transverse Mercator 3D is read from the Dataset's own description

Session 2296. Status: **accepted**. Context: the EPSG Dataset's record of method 1111, fetched from its
public API on 2026-11-28; projected system 9895 and conversion 9894; `doc/todo/79` row 27.

## Decision

The September 2019 revision of Guidance Note 7-2 has no section for method 1111. The Dataset's
record states: "For the calculation of easting and northing from latitude and longitude, the formulas
for the Transverse Mercator projection (method 9807) are followed. The third coordinate, ellipsoidal
height (h), is carried across unchanged". The reverse is stated the same way. So
`Method::TransverseMercator3D` is evaluated by section 3.2.3.1's formulas. A projected system here has
two axes, so the height never enters. A geospatial measure's points are positions on the page.

The record's example, LUREF / Luxembourg TM (3D) (EPSG 9895), is the test: 49°34′00.7381″N
5°56′16.1460″E is (63444.834, 70387.372) m. One transcription slip in its parameter lines is read here,
and it changes no value. The radians printed beside the origin's latitude and longitude (49°50′00″N
and 6°10′00″E, conversion 9894's 49.5 and 6.1 in EPSG unit 9110) are 0.865105111 and 0.103634480.
Those are the point's radians, printed again two lines below, and not the origin's (0.869755744 and
0.107628637). The test states the origin as the conversion does.
