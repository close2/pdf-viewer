# Round 2294 — eleven projection methods are built

Lane 3. Subject: `doc/todo/79`'s rows 12 to 18, continuing round 2293. ADRs 3877–3880.

- **Built** (ADR 3877), each in both directions: Equal Earth (3.4.4, EPSG 8858's example), Lambert
  Cylindrical Equal Area and its spherical form (3.2.6 names USGS Professional Paper 1395: its two
  normal-aspect examples), Equidistant Cylindrical and its spherical form with the codes they
  replaced (3.2.5, EPSG 4087's example; 3.2.5.1 by its formulas), Bonne in both orientations (no
  example printed: the paper's), Laborde Oblique Mercator (3.2.4.2, EPSG 29701 and 8441, and the
  comparison table), and the Transverse Mercator Zoned Grid System (no example: each UTM zone's own
  system). Parameters 8830 and 8831 are new.
- **Checked by dimensions and derivation**: Equal Earth's denominator is the northing polynomial's
  derivative; every fraction of 3.2.5's arc series to `e¹⁴` equals an exact expansion of its
  equation (1); the reverse series in `n` round-trips to 1e-11° at every tenth of a degree.
- **Read** (ADR 3878): Laborde's printed `H1` is not its Newton step at `k = 0` (both starts are
  tested to reach the printed root); its stopping test bounds only the real part; the reverse
  example's `L` slips a decimal; the comparison table's second row has its columns transposed.
  (ADR 3879): the zoned grid's easting is printed in the older series form, `INT` is read as `floor`,
  and the reverse zone is the easting's millions, a choice. (ADR 3880): 9842 and 9823 state ϕ1 as a
  latitude of natural origin; on an ellipsoid the spherical form's `R` is `RC` at ϕ1, so WGS 84 /
  Plate Carree uses the semi-minor axis.
- **Not built**: Local Orthographic (1130), which has no section in the 2019 revision; it waits on
  `doc/questions/Q1114` with 1125 and 1102.
- Covered: 5758 of 5780 projected systems (ADR 3823's command); 22 left over 12 methods. Ledger:
  12.10.4 stays `partial`, note, code and tests moved.

**Gates.** Over `-p pdf-model -p viewer-core -p conformance`: fmt 0; clippy 101 (two
`arithmetic_side_effects`, `match_same_arms`, `too_many_lines` twice, `clone_on_copy` three times,
`large_types_passed_by_value`), fixed, 0; `pdf-sandbox` 0; nextest 4302 run, 2 failed
(`repair_census` on a clamp, `records` on this paragraph), fixed, then 4302 passed; `audit-diff` 0
expected; rustdoc 0; fuzz 0; `cargo test -p conformance` 0 (485 passed); launch 0 (6 documents, 36 figures banded, 0 outside).

## Habit

Where a worked example sits next to a comparison table, run the example's formulas over the
table too: a row that matches the other method's column to the millimetre is a transposition,
not a bug in either implementation.
