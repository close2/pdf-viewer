# 3880 — Equidistant Cylindrical's replaced codes and its sphere are read

Session 2294. Status: **accepted**. Context: Guidance Note 7-2 sections 3.2.5, 3.2.5.1, the
implementation notes and the revision history (version 39, April 2009); ADR 3877.

## Decision

- **The replaced codes.** The carried table states three systems by 9842 (Equidistant Cylindrical)
  and 9823 (Equidistant Cylindrical (Spherical)). The September 2019 revision names neither. Its
  history says version 39 "updated formula to hamonise parameters and symbols with similar methods",
  and the current codes 1028 and 1029 carry the same names. The replaced codes state a latitude of
  natural origin (8801) where 1028 and 1029 state the latitude of the first standard parallel
  (8823); both formulas have one latitude parameter, ϕ1. So 9842 and 9823 are evaluated by the
  formulas of 1028 and 1029, the latitude of natural origin taken as ϕ1, by code in the table and in
  a WKT string (`projection::parallel_from_natural_origin`). Every carried system states zero, so
  the reading decides no carried value; a WKT test shows a non-zero one taken as the parallel.
- **The sphere's R.** Section 3.2.5.1 says that on an ellipsoid `R` is the conformal sphere's at ϕ1,
  the implementation notes' `RC = a √(1 – e²) / (1 – e² sin² ϕ)`. At ϕ1 = 0 that is the semi-minor
  axis, so WGS 84 / Plate Carree (EPSG 32662) takes `b`, and the test says so.
- **Two names.** Section 3.2.5's example calls `n` the "second flattening"; the formula
  `n = [1 − (1 − e²)^½] / [1 + (1 − e²)^½]` is the third flattening, the series' own variable. Section
  3.2.5.1 says "the latitude of natural origin (ϕ1)" of the parallel its formulas use. Both
  are names only; the formulas are evaluated as printed.
