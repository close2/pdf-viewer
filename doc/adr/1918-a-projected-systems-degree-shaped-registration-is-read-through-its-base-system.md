# 1918 — A projected system's degree-shaped registration is read through its base system, and fitted in its plane

Session 1541. Status: **accepted** and **built**, on the owner's answer `doc/questions/A271`.
Amends ADR 1593 §1 (a projected map is no longer refused by its points' shape) and settles the
question ADR 1593 §2 left open (interpolation in the projected space). Builds on ADRs 1586, 1587
and 1672. Context: ISO 32000-2 §12.10.2 (Table 269's `/GPTS` and NOTE 2), §12.10.4; IOGP Guidance
Note 7-2 for the projections.
Code: `crates/pdf-model/src/geospatial/{mod,registration}.rs` (`Geospatial::registration_map`,
`Geospatial::registration_in_plane`, `AffineRegistration::fit_in_plane`),
`crates/pdf-model/src/measurement.rs` (`Viewport::page_position`), `crates/viewer-core/src/located.rs`,
`crates/pdf-model/examples/geospatial_census.rs`. Tests: `geospatial::tests::projected_registration_points_shaped_as_degrees_are_the_base_systems_degrees`,
`geospatial::tests::a_projected_registration_is_fitted_in_the_plane_and_run_backwards`,
`crates/viewer-core/tests/measuring.rs`; `tools/drive-windows.sh` step `38-located`.

## 1. The sentence departed from

Table 269's `/GPTS` is "[a]n array of numbers that shall be taken pairwise, defining points in
geographic space as degrees of latitude and longitude, respectively when defining a geographic
coordinate system. These values shall be based on the geographic coordinate system described in
the GCS dictionary. When defining a projected coordinate system, this array contains values in a
planar projected coordinate space as eastings and northings." NOTE 2 beside it: "Any projected
coordinate system includes an underlying geographic coordinate system."

The third sentence carries no modal verb, and it is still a requirement on this program: it is
the definition of what a conforming file's numbers *mean*, and the only party that acts on a
meaning is the one that reads it. ADR 1622's non-departures — a `should` declined, a `may` not
taken, a NOTE, a method offered to another kind of processor — do not include reading a conforming
file's values as something the table says they are not. So it is recorded as a **departure**, as
the owner's answer names it.

**What is read instead**: a projected `/GCS` whose every `/GPTS` pair lies within ±90 by ±180 —
the shape of a latitude and a longitude — is read as the base system's degrees, which is the
reading NOTE 2 makes available and the second sentence's "based on the geographic coordinate
system described in the GCS dictionary" describes. A set with one pair outside the shape is read as
the table states, eastings and northings, every pair.

**The cost**: a conforming file whose every easting lies within 90 units and every northing within
180 units of its grid's origin is read as degrees and placed elsewhere. For a system whose false
origin offsets its grid — a UTM zone's 500 000 m false easting, each State Plane system of
`every_method_closes_at_a_census_maps_registration_point` — those grid points lie outside the
system's own area; for one whose origin is not offset, they are a map under 360 units across at the
origin itself. **The census finds no such file**:
no document of the crawl or the corpora writes eastings and northings at all.

**The gain, at HEAD before the build** (`examples/geospatial_census.rs`, 90 763 files, 136 s,
7.85 GiB peak): 158 documents state a projected `/GCS`, every one degree-shaped and none with
`/PCSM`; **151 of them were refused by `RegistrationShapedAsDegrees`** and 7 by their method or
their WKT first, which this departure does not reach. A271's "158 gained" is the population; 151
is what this tree gains, and none is lost.

## 2. Between the points, the map is the plane's

ADR 1593 §2 reads a position between registration points through the affine map they determine,
and left "interpolation in the projected space" open for exactly these maps. It is taken: **a
projected system's registration is fitted in the system's plane**, each degree-shaped point first
carried there by the forward projection (ADR 1672), and a position between the points is the
plane's, carried to the earth by the inverse. The page of a projected map is a picture of the plane
— that is what §12.10.4's system is, the transform "between geographic coordinates and a
two-dimensional (projected) coordinate system" — so the plane is where the page is linear, and an
affine fit in degrees would bend every map whose parallels curve on it. Points stated as eastings
and northings are fitted in the same plane, where they are stated; a geographic system's map stays
affine in its degrees, ADR 1593's reading of §12.10.4's "direct mapping".

The departure is still said in degrees of either axis — each registration point the file states
against the inverse of where the plane's map puts it — so a host writes the same number whatever
the system. `Viewport::page_position` runs the same map backwards: the forward projection, then the
plane's map solved (ADR 1672 §5).

## 3. Consequences

`Refusal::RegistrationShapedAsDegrees` is gone; `Geospatial::registration_map` is the one map both
directions read, and `viewer_core::located` asks it rather than fitting its own. The window that
showed Q271's refusal on the drive's UTM map now shows a position inside its rectangle of degrees.
§12.10.2's row records the departure; a `/DCS` on another datum is that row's other unexecuted
requirement and is not touched here.
