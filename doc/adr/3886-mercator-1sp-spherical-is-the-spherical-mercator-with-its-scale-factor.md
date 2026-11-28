# 3886 — Mercator (1SP) (Spherical) is the spherical Mercator with its scale factor

Session 2296. Status: **accepted**. Context: Guidance Note 7-2 section 3.2.1.1 (method 1026); the
EPSG Dataset's records of methods 9841 and 1026, fetched from its public API on 2026-11-28; `doc/todo/79`
row 22.

## Decision

The carried table's one 9841 system is Popular Visualisation CRS / Mercator (EPSG 3785). The
September 2019 revision prints no section for 9841. The Dataset's record for it holds no formula
("See information source"). Its deprecation says that 1026 replaced it, with the reason "Removed scale
factor as defining parameter for spherical mercator".

So 9841 is read as section 3.2.1.1's formulas with a scale factor `kO` multiplying `R`, as section
3.2.1's variant A has it multiply `a`. 1026, which states no scale factor, is evaluated with `kO = 1`.
Both codes name one `Method::MercatorSpherical`, by code and by either name in a WKT string. `R` is the
sphere's radius. On an ellipsoid it is the section's conformal sphere at ϕO, the implementation notes'
`RC`, and the 88° limit the section gives ("should not be used poleward of 88º") is refused in both
directions, as for the Pseudo-Mercator.

3785 states `kO = 1` on a sphere of WGS 84's semi-major axis, so the readings agree on every value
the table carries. The test is the section's example on its sphere, R = 6371007 m. It also checks
3785 against WGS 84 / Pseudo-Mercator (3857): section 3.2.1.2 states the Pseudo-Mercator as the same
formulas on that radius.

If a 9841 system with `kO ≠ 1` ever appears, its reading is this ADR's. Nothing the table holds can
tell the reading wrong.
