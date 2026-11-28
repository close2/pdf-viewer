# Round 2293 — a shared holder is restated, and seven methods are built

Lane 3. Subject: the last redaction resource shape ADR 3869 left (a direct entry of a `/Resources`
object the page names and shares), the copy-and-keep sweep, and `doc/todo/79`'s rows 6 to 11.
ADRs 3873–3876, Q1114.

- **The probe first**: five fixtures, each with two pages, where page one paints a direct shading
  or pattern under the region and page two never does. The shapes: both pages name one
  `/Resources` object (shading, pattern), or one `/Shading` table; a form's `/Resources` is the
  object the other page names; a form with no `/Resources` draws from the page's own. Before the
  change, the producer's sample table was in each output (five tests failed).
- **The holder is restated** (ADR 3873): the resources the edited stream resolves in, the form's or
  the page's chain. A holder only that stream names is rebuilt with it. One something else names
  is restated in place when nothing else draws the entry. A form without resources gives the page
  its replacement. Each test compares page two's pixels before and after. The keeping case (page
  two paints it) keeps the table.
- **The sweep is closed** (ADR 3874): every way the walk keeps a producer's object is listed with
  the ADR that answers it. What stays is the search counting as drawn what it cannot read.
- **Seven method codes** (ADR 3875): Lambert Azimuthal Equal Area Spherical (3.3.2.1 prints no
  formulas and names USGS PP 1395, which was fetched; its R = 3 example, EPSG 3408 and 3973),
  American Polyconic (PP 1395's ellipsoid example, 5880 by round trip), 2SP Michigan (EPSG 6201),
  and Krovak in four forms (2065, 5221; the Modified forms on their printed parameters, since PROJ's
  SQL holds seven parameters a conversion and 5224 and 5225 are refused by the first one missing).
- **Readings** (ADR 3876): Polyconic's equator case is `A = 0`, its `H` is `M′`, Krovak's `D` the
  `atan2`, each tested on both sides. **Not built**: 1125 and 1102, not in the 2019 text (Q1114).
- Covered: 5741 of 5780 projected systems (ADR 3823's command), 4 refused by a parameter. Ledger:
  12.10.4 `partial`, note and tests moved; 12.5.6.23 `departed`, six tests added.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-transform -p viewer-core -p conformance`: fmt 0;
clippy 101 three times (`unnecessary_qualification`, `match_same_arms`, `too_many_lines` twice,
`arithmetic_side_effects`, `large_types_passed_by_value`, `type_complexity`, `single_match_else`),
fixed, 0; `pdf-sandbox` 0; nextest 6209 run, 3 failed (`repair_census` on three `.ok()`
defaults, `names` on a doc path a new `Coefficient` variant made checkable, `records` on the
length), fixed, then 6209 passed; `audit-diff` 0 expected; rustdoc 0; fuzz 0; `cargo test -p
conformance` 0 (485 passed); launch 0 (6 documents, 36 figures banded, 0 outside).

## Habit

Check a section's formulas against the paper it names: where no example is printed, dimensions
and the source's own equation are what catch a misprint.
