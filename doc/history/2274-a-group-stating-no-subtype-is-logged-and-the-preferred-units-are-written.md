# 2274 — A `/Group` stating no subtype is logged, and `/PDU`'s units are written

Lane 3, the leftovers of rounds 2271 and 2272. ADRs 3797–3800, Q1095.

- **`transparency_group`** (ADR 3797): new class `model/group-attributes-unusable` (§8.10.3, Table
  94), mapped to section 5.1 in `pdf-archive/src/repairs.rs`, in the grammar's `COVERAGE`. Every
  `/Group` read of the content walk goes through `transparency_attributes`: a form's at the content
  that invoked it (read before the stream becomes the form's), the page's own (five reads) at the
  page object. Two `repair_classes.rs` tests, form and page.
- **The `?` sweep**: fifteen one-line `?` reads on an `Option` in `pdf-model/src/content`; three are
  this class, the other twelve are not repairs, each said in ADR 3797.
- **D01** (ADR 3798, Q1095): priced, not built. PROJ 9.9.0's EPSG v13.102: 902 geographic, 5417
  projected, about 360 KB of `static` data; the eight built methods cover 95.2 %. IOGP's terms allow
  subsets but forbid distribution for profit and bind recipients, a second licence over part of an
  Apache-2.0 program — the owner's call. 12.10.3, 12.10.4 and the 12.10 head now `partial`.
- **D02** (ADR 3799): `/PDU` writes the geospatial sentence — angular (degrees or grads) on every
  position, linear on a projected `/DCS` position through `Displayed::Projected`'s new
  `metres_per_unit` (carried on the confined protocol); area has nothing to apply to, said in the
  row; an unlisted name is said. Three tests, expected text from the units' definitions.
- **Annex O on `/Rotate`** (ADR 3800): the code is right — the corner is default user space's;
  a `/Rotate 90` fragment test pins it. O.2.2's note says so.
- 2271's `findings.json`: D01 and D02 `fixed`. No owner audit file edited.

**Census.** `repair_census.rs` and the content-class gate pass on the new lines. `tools/state.sh
--round 2274 launch` (pre-rebase tree `3f09762dd`): 6 documents, 36 figures banded, 0 outside.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-archive -p viewer-core -p viewer-host -p
viewer-confined -p viewer-qt -p conformance`, rebased on `47acf9813`: fmt 0, clippy `-D warnings --all-targets` 0,
`pdf-sandbox --bins` 0, nextest `--lib --bins --tests` (`xvfb-run -a`) 5739 of 5740 — the one this
record before its gates were written, as `audit-diff.py` shows, against an empty manifest; rustdoc
of `pdf-model`, `pdf-archive`, `viewer-host`, `viewer-confined` 0 and fuzz 0 (both pre-rebase); `cargo test -p
conformance` 0.
