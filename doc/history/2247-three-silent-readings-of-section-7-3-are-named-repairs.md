# 2247 — Three silent readings of §7.3 are named repairs, and a soft mask that does not decode is logged

Lane `gwg`, the silent-repair campaign's `pdf-syntax` pass over audit 2243, and the census's fourth
pass (ADRs 3574, 3669). ADRs 3689 to 3691.

- **B02 to B04** (ADR 3689). New classes: `syntax/stream-not-indirect` (§7.3.8.1, a stream as an
  array element or a dictionary value), `syntax/name-holds-null` (§7.3.5, a `#00` escape) and
  `syntax/object-number-spelling` (§7.3.10 under issue #379's grammar, for a reference and an
  object header; object number 0 too). Each is read as before and logged; each maps to section 5.1
  alone. The converter's answer is ADR 3521's: the rewrite answers the spelling; the nested stream
  and the null are written back as read, so the output's own check names them. Inside content,
  `name-holds-null` takes the fence's general sentence, since lane 3 is editing
  `IN_CONTENT_BY_CLASS`. The three manifest entries are gone and 2243's findings say fixed;
  `doc/errata-read.md`'s #379 row is no longer owed.
- **B02's writer half** (ADR 3690): `write::object` states every stream's `/Length` as the bytes
  it writes; the nesting is kept. Hoisting a nested stream into an object of its own is left.
- **Census** (ADR 3691), `HELD` 373 → 360: `image/soft_mask.rs` 8 → 0, `substitute.rs` 3 → 0,
  `image/dispatch.rs` 4 → 2. New classes `model/soft-mask-undecodable` (§11.6.5.2; not for this
  reader's own bounds), `model/image-depth-absent` (§8.9.5.1; not behind `JPXDecode`) and
  `font/descriptor-style-unusable` (§9.8.1).
- Left, in order: `rich_text/markup.rs` 6, `jpeg2000.rs` 6, `geospatial/system.rs` 6 — none of the
  three readers holds the document or returns what it read past; a nested stream hoisted by the
  assembly; a sentence of its own for `name-holds-null` in `IN_CONTENT_BY_CLASS`.
- **Census**, with and without the pack: no line names a new class, and the conforming counts are
  2242's; the other lines differ from 2242's census by the rounds merged into the base since.
  veraPDF 1.30.3 over the 5505 outputs written as conforming: 40 not compliant and 12 unfinished,
  as round 2242's.
- Launch: the first three versions of the B04 check put WTPDF's `open_kinstructions` outside
  (10 409.2, 10 290.4, 10 228.6 against 10 200.0; main 10 094.5). Callgrind of reading WTPDF's
  objects found the cost in the parser's re-reading and in the `#00` test inside the name loop;
  the lexer now marks a signed or padded integer and escapes are read out of line (ADR 3689).
  `tools/state.sh --round 2247 launch` then: 6 documents, 36 figures banded, 0 outside (WTPDF
  10 121.4). Two opens fell below their floors and their bands were moved by the file's rule.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-font -p pdf-archive -p pdf-transform -p
viewer-qt -p conformance`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0,
nextest `--lib --bins --tests` (`xvfb-run -a`) 6254 of 6254 (`audit-diff.py`: "failed 0, expected
0"); rustdoc of the four reader crates 0; fuzz 0. After the launch fix: fmt, clippy and nextest of
`pdf-syntax` 0 (454 of 454), `cargo test -p conformance` 0.
