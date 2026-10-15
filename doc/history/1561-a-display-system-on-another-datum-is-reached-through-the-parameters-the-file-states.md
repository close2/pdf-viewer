# 1561 — A display system on another datum is reached through the parameters the file states

Slot 4 of batch eighty-nine, 2026-10-15, a clause round on the owner's answer `doc/questions/A436`.
ADR 1958; no question. **Ledger**: §12.10.2 `partial` → `departed` (A271's sentence, ADR 1918, is
the one left); §12.10 `partial` → `implemented` with it; §12.10.3's note names the bound CRS.

**Premise.** Held: §12.10.2 was `partial` on the datum alone, refused by `DatumTransformation`, the
Note in `doc/md/`. **Hypothesis.** One function over seven parameters, its reverse the sign change:
held (section 4.2.3's EPSG reversal). A handful of census files with parameters, one a witness: did
not hold. Over 90 763 files, 127 s, one states a `TOWGS84` (`0300898.pdf`, all zeros, in its `/GCS`)
and none a `/DCS` structure, so no census map displays differently and no witness can be driven.

**Built.** `geospatial/datum.rs`: `Helmert` (section 4.2.3, Position Vector), `DatumShift`, and
`transform`, section 4.3.1's geog2D concatenation with zero height (4.1.4) and section 4.1.1's
geocentric conversions, now `Ellipsoid::geocentric` and `::geodetic`. Two systems meet on a datum
both reach; WGS 84 is the one datum known by name, and on its figure. `system.rs` reads the older
form's `TOWGS84`, as OGC 01-009 defines it (sections 7.3.18, 10.4, 12.3.27; fetched, held at
kept outside the tree, its provenance and digest recorded in `doc/third-party-data.md`). It also reads ISO 19162
section 20's `BOUNDCRS` with a Helmert-family method, units per section 20.2.3. A grid method is
refused by its name; a statement that cannot be read is kept and raised only when a datum change
asks. Tests: the Note's examples of sections 4.1.1, 4.2.3 (both conventions) and 4.2.4, and ISO
19162 section 20.3's examples 1 and 3; a sign planted in `Helmert::apply` fails two (trap 13). The
census counts both statements, calibrated on a stated and an unstated scratch map.
**Found.** Section 4.2.3's printed `XT` = 3657660.78 m is 6 mm from its formula's 3657660.774 m,
either rotation matrix; the test's budget is the last printed digit, and says why.

**Files.** `crates/pdf-model/src/geospatial/{datum,ellipsoid,system,mod}.rs`,
`crates/pdf-model/examples/geospatial_census.rs`, ADR 1958, `doc/conformance/ledger.toml` (§12.10,
§12.10.2, §12.10.3), `doc/todo/65-the-remaining-frontier.md`, `doc/state-of-play.md`,
`doc/crate-map.md`, `doc/third-party-data.md`, this record.

**Gates.** rustfmt `--check` exit 0 on the five Rust files. In the export (HEAD plus these paths,
the round's own export): clippy `-D warnings -p pdf-model --all-targets` exit 0; nextest `-p
pdf-model` exit 0, 2 080 passed, 21 skipped. The worktree's own pdf-model test build stops on a
sibling's `tests/page_tree_nodes.rs`. `cargo test -p conformance --no-fail-fast` exit 101, 434
passed, 2 failed on slot 5's record 1562. One lock hold of 466 s, exit 0, peak 8.43 GiB, held the
census (exit 0) and the arms: cpu-1x, gpu-1x and compute-1x, each exit 0, 968 of 968 pages with
digest and error equal by page to HEAD's arms. The 4x arms were not run: no render crate
names `geospatial`, and four rounds queued for the first lane. `tools/state.sh docs` not owed
(pdf-model is not held). No drive: no host changed, and no census document is a witness.
