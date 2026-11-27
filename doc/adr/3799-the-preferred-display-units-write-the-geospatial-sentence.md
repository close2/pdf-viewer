# 3799 — `/PDU`'s preferred units write the geospatial sentence, and its area unit has nothing to write

Session 2274, review 2271's D02. Status: **accepted**. Context: ADR 1593, ADR 1678; ISO 32000-2
§12.10.2 Table 269.

## Context

Table 269's `/PDU` is "[a]n array of three names that identify in order a linear display unit, an
area display unit, and an angular display unit", and lists the valid names: `M`, `KM`, `FT`, `USFT`,
`MI`, `NM`; `SQM`, `HA`, `SQKM`, `SQFT`, `A`, `SQMI`; `DEG`, `GRD`. The units were read and carried
to the host and applied to nothing, while the 12.10.2 row listed `/PDU` in "Table 269 entire". The
table states no `shall` on them, so this is a choice, written down here.

## Decision

`viewer_host::measuring`'s geospatial sentence writes in the units `/PDU` prefers:

- **Angular**: every latitude and longitude it writes — the file's system's and `/DCS`'s — in degrees
  or, for `GRD`, in grads of "1/400 of a circle, or 0.9 degrees", six places for both.
- **Linear**: a projected `/DCS`'s easting and northing, converted from the system's own unit through
  metres. `Displayed::Projected` therefore carries the system's metres per unit beside its name, and
  the confined renderer's protocol carries it across. Each unit's length is its definition (the
  international foot 0.3048 m, the U.S. survey foot 1200/3937 m, the international mile 1609.344 m,
  the international nautical mile 1852 m), and each is written to the places that keep a centimetre,
  the resolution ADR 1678 gave the metre.
- **Area**: nothing the sentence writes is an area. The geospatial reading gives positions; no area
  on the earth is computed from a traced path, so the area unit is read and has nothing to apply to.
- A name that no list of the table holds is said in the sentence, and the system's own unit is used.

## Consequences

- A geodesic length or area of a traced path, if one is ever built, takes the same table.
