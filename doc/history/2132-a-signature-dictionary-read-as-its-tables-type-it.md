# Round 2132 — audit 2128's groups C, B and D: a signature dictionary read as its tables type it

A fix round in the `gwg` worktree for review 2128's remaining findings: B03, B04, B05, B06, B07,
T02 and D03.

## What changed

- C (ADR 3229): `byte_range` refuses a `/ByteRange` with an odd member or a member that is no
  non-negative integer (B04, `Coverage::Malformed`); a usage-rights `/V` is understood only absent
  or as the name `2.2` (B05); `must_cover_whole_file` holds a `/Type /DocTimeStamp` whatever its
  `/SubFilter` (B06).
- B (ADR 3231): `pades_departures` reads the signed content-type attribute. Absent is the new
  `PadesDeparture::NoContentTypeAttribute`; a value other than id-data, or an eContentType other
  than id-data, is `ContentTypeIsNotData` (B03). Both comments that claimed presence was checked
  now are true (D03).
- D (ADR 3230): `Signature::covers_its_revision` reads Table 255's "entire PDF file" as §12.8.1
  does for several revisions — up to the `%%EOF` ending the signature's update. `pades_departures`
  and `viewer-core`'s report ask it, so a DSS revision §12.8.4.3 requires is no departure (B07).
  Both sides are tested (T02): `viewer-core/tests/pades_range_end.rs` and
  `a_pades_range_that_stops_inside_its_revision_is_a_departure`.
- Two unit tests follow the reading: `a_pades_signature_is_held_to_the_rules_that_need_no_certificate`
  (a byte after the final EOL is no short range) and the reworded sentence's notes test.

## Ledger and manifest

The six rows 2128 named (12.8.1, 12.8.2.3, 12.8.3.3.1, 12.8.3.4.2, 12.8.3.4.3, 12.8.5.2) stay
`implemented`: no B finding contradicts them now. Five notes are restated and list the new tests;
12.8.3.3.1 is unchanged (round 2130 fixed its findings). The manifest loses the six entries for
B03 to B07 and is empty of failures; 2128's `findings.json` marks the seven findings fixed.

**Gates.** Over `-p pdf-signature -p viewer-core -p pdf-transform`: fmt 0; clippy 0 (second run,
an unfulfilled `expect` in the new test); nextest 2586 passed, difference against the manifest 0
and 0; rustdoc 0; `cargo test -p pdf-signature` 0; fuzz `check --bins` 0; `cargo test -p
conformance` 0 on the second run, after a quotation gained its clause. Left: `timestamp.rs`'s outermost-link rule
(`ChainRefusal::RangeDoesNotCoverTheFile`) still measures a document timestamp to the file's end.

## Habit

A `shall` read without the clause's sentence about several revisions turned the standard's own
long-term-validation shape into a departure. Before reporting a range as short, ask §12.8.1.
