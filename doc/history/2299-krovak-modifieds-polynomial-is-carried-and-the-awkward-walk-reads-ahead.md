# Round 2299 — Krovak Modified's polynomial is carried, and the awkward-classes walk reads ahead

Lane 3. Subject: ADR 3875's carried Krovak Modified systems, and ADR 3826's longest small walk.
ADRs 3897 and 3898.

- **Krovak Modified** (ADR 3897). PROJ 9.9.0's `data/sql/` was searched whole and holds no row of
  8617, 8618 or 1026 to 1035 for 5219, 5220, 5511 or 5512. IOGP's registry API
  (`apps.epsg.org/api/v1/Conversion/<code>/`, no account or form) serves all nineteen.
  `tools/epsg-table.py --fetch` saves the four records. The generator pins the SHA-256 of each
  parameter list, requires the first seven to equal PROJ's, and appends the rest as the API spells
  them. Both refusals were planted and seen. Unit 9203 (coefficient) joins the table. EPSG 5224,
  5225, 5515 and 5516 resolve and reproduce section 3.1.2.3's and 3.1.2.4's grid point to the
  centimetre, and the reverse to the millisecond of arc. The polynomial is about 8 cm there, so
  without it the test fails. The Greenwich forms take the point 17°40′ west of Ferro. ADR 3823's
  command leaves 13 of 5780 systems over 3 methods. None is now refused by a parameter.
- **The walk** (ADR 3898). Instrumented by root, document and class. Cold, 442 s of 462 s went to
  classifying the population and 20 s to sweeping. Warm, the population took 10 s. The time is the
  corpus's cold scattered 4 KiB reads on the rotating disk. Sixteen threads now read each root's
  sample whole before it is classified, and `population` is unchanged. With the sample evicted
  first (`POSIX_FADV_DONTNEED`), population 448.6 s became 118.1 s, the walk 467.3 s became
  133.6 s, and the hold 508 s became 136 s. The 18 count lines are identical.
- Ledger: 12.10.4 stays `partial`, note moved (no parameter refusal; the registry route).
  `doc/todo/79`, `NOTICE` and `epsg/mod.rs`'s module comment say where the twelve come from.

**Gates.** Over `-p pdf-model -p viewer-confined -p viewer-core -p conformance`: fmt 0; clippy 0,
twice; `pdf-sandbox` 0; nextest 4448 run, 1 failed (`evidence`: ADR 3897 named a scratch path;
reworded to the command), then 4448 passed; `audit-diff` 0 expected, 0 failed; rustdoc 0
(`pdf-model`, `viewer-confined`); fuzz not owed (no public type changed); `cargo test -p
conformance` 0; generator `--check` 0; launch 0 (6 documents, 36 figures banded, 0 outside); the walk 0
killed in each of four runs.

## Habit

Before you optimise a corpus walk, run it twice in a row. If the second run is many times faster,
the time is the page cache's, and the parser is not the place to look.
`posix_fadvise(DONTNEED)` makes the cold run repeatable without privilege.
