# Round 2296 — nine projection methods are built

Lane 3. Subject: `doc/todo/79`'s rows 19 to 27, continuing round 2294. ADRs 3885–3888.

- **Built**, each in both directions and tested on its worked example through its one system:
  Hyperbolic Cassini-Soldner (3.2.2.1, EPSG 3139, in links), Modified Azimuthal Equidistant
  (3.3.3.1, 3295), the Guam Projection (3.3.3.2, 3993), Mercator (Spherical) and the code it
  replaced, 9841 (3.2.1.1's example on its sphere; 3785 against 3857), Lambert Conic Conformal (2SP
  Belgium) (3.1.1.4, 31300), Lambert Conic Near-Conformal (3.1.1.6, 22700), the New Zealand Map Grid
  (no formulas printed: the Surveyor-General's Technical Report 4.2 and its three test points,
  27200), the Tunisia Mining Grid (3.4.2, 22300) and Transverse Mercator 3D (no section: the
  Dataset's own description and example, 9895). No new parameter codes.
- **Checked**: every printed constant of the near-conformal example to its digits, its arc against
  section 3.2.2's meridian distance to a millimetre; the Tunisia rates against the decree's six
  tabulated ends; the hyperbolic divisor by dimensions (a length in chains).
- **Read** (ADR 3885): the hyperbolic divisor's unit and the example's unsigned `ϕ1'`; the modified
  azimuthal distance as the sine rule's quotient with `sin α` divided out, after the printed
  quotient failed beside the meridian (478 m at a nanodegree). (3886): 9841 as 1026 with `kO`. (3887):
  the NZMG source; the near-conformal `θ′` signed as `r′`, and its exact reverse; the Tunisia text's
  ϕ for λ. (3888): the Dataset example's origin lines carry the point's radians.
- Covered: 5767 of 5780 projected systems (ADR 3823's command), 46 method codes; 13 left over
  1125, 1102 and 1130, all waiting on `doc/questions/Q1114`. Ledger: 12.10.4 stays `partial`,
  note, code and tests moved. The refusal tests now name WGS 84 / Equi7 Europe (27704).

**Gates.** Over `-p pdf-model -p viewer-core -p conformance`: fmt 0; clippy 101
(`too_many_lines` twice, `Projection::new` and `forward`, split into `required_parameters` and
`plane_forward`), then 0; `pdf-sandbox` 0; nextest 4321 run, 1 failed (`names`: a struct `Arc`
of this change made three `Arc::` paths in doc comments look undeclared; renamed), then 4321
passed; `audit-diff` 0 expected; rustdoc 101 (a bracketed `[i]` in a quotation), escaped, 0; fuzz 0;
`cargo test -p conformance` 0 (485 passed); launch 0 (6 documents, 36 figures banded, 0 outside).

## Habit

Where a section prints a special case for a divisor that vanishes on a line, evaluate the general
formula a nanodegree from the line: if it is `0/0` there, the special case is not the only place the
formula needs one.
