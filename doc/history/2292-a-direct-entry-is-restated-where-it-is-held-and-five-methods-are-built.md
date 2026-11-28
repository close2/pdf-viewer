# Round 2292 — a direct entry is restated where it is held, and five methods are built

Lane 3. Subject: the shape ADR 3861 left (a shading or pattern written directly in inherited
resources), and `doc/todo/79`'s rows 1 to 5 (A1095). ADRs 3869–3871.

- **The probe first**: two pages inherit `/Resources` stating `/P0 << /PatternType 2 /Shading
  << … /Function 7 0 R >> >>` (or `/Sh0` directly); page one paints it under a region, page two
  never does. Before: the producer's sample table was in the output (two tests failed on it).
- **The holder is restated** (ADR 3869, `redact/undrawn.rs`, `redact.rs`): the entry's holder is
  the nearest indirect object above it — the `/Pages` node, or the resource dictionary or category
  table it names — checked against the page's effective resources. The search counts an equal
  dictionary reached by any other root as drawing it; one nothing else draws has its holder's slot
  reserved and written with the page's replacement at the path, every other entry carried. The
  other page's pixels are compared before and after in each test; a keeping case (the other page
  paints it) and a holder named by reference are tested too.
- Left: a direct entry of a `/Resources` object the page itself names and shares (§7.7.3.3).
- **Five methods** (ADR 3870): Colombia Urban (3.4.3, EPSG 6247), Transverse Mercator South
  Orientated (3.2.3.3, EPSG 2053, held to 1.5 cm since its point is printed to 0.001″), Hotine
  variant B (3.2.4.1, EPSG 29873) and A (the same example at the natural origin, built from its
  parameters), Lambert Conic Conformal West Orientated (3.1.1.3, no example printed: EPSG 2297
  against the 1SP formulas). New parameters: azimuth (8813), rectified angle (8814), origin height
  (1039); 8811/8812/8815/8816/8817 take their natural-origin roles.
- **A reading of the text** (ADR 3871): Hotine's special case at αC = 90° adds |uC| east of λC as
  printed — 20 000 km for a point a metre east of LV03's centre. The general formula is evaluated
  for every azimuth, the special `u = 0` at λ = λC kept; tested on EPSG 21781.
- Covered: 5716 of 5780 projected systems (ADR 3823's command), 64 over 29 methods left.
- Ledger: 12.10.4 stays `partial`, note and tests moved; 12.5.6.23 `departed`, four tests added.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-transform -p conformance`: fmt 0; clippy 101
(`manual_midpoint` three times, `large_enum_variant` on `ReferenceSystem`, `clone_on_copy`,
`arithmetic_side_effects`), fixed, 0; `pdf-sandbox` 0; nextest 5512 run, 5510 passed, 2 failed —
`records` on this record's unwritten Gates paragraph, and `bounded`'s self-test under load (it
passes in the conformance run); `audit-diff` 0 expected; rustdoc of `pdf-model` and
`pdf-transform` 0; fuzz 0; `cargo test -p conformance` 0 (485 passed); launch 0 (6 documents, 36 figures
banded, 0 outside).

## Habit

Evaluate a printed special case on both sides of the line it splits: one that holds on one side
only is a defect in the text, and the continuity the method defines decides it.
