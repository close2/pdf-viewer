# 1541 — A projected map's degrees are read through its base system, and fitted in its plane

Slot 2 of batch eighty-six, 2026-10-14, a clause round on the owner's answer A271. ADR 1918;
question Q436. §12.10.2 stays `partial` with one requirement left instead of two; §12.10.4's note
names the departure.

**Premise.** The refusal is `geospatial/mod.rs:141` and returned at 306; 493 is a test's assertion.
**The row is §12.10.2's (ledger line 4726), not §12.10.3's**: Table 269 is §12.10.2's table, and
§12.10.3's row (Table 270) is `implemented` and untouched. **Hypothesis.** The census is
`examples/geospatial_census.rs`; re-run at HEAD it finds the 158 projected maps, every one
degree-shaped, but **151** were refused by the shape — 7 by their method or WKT first — so the
tree's gain is 151 documents, not 158. None is lost: no file writes eastings and northings.

**Built.** `Geospatial::registration_map`: a projected system's registration is fitted in its
plane, degree-shaped points carried there by the forward projection, a position between them
carried back by the inverse; a geographic system's map stays affine in degrees (ADR 1593). The
departure is still said in degrees. `Refusal::RegistrationShapedAsDegrees` is gone;
`Viewport::page_position` and `viewer_core::located` read the one map. Drive step `38-located` now
expects a position on the projected map. The census asks `/DCS` too, and finds none in 90 763 files
(calibrated on `drive-displayed.pdf`), which prices the row's last requirement for Q436.

**Not settled.** §12.10.2 cannot be `departed` while a `/DCS` on another datum is unexecuted, and
A171 did not decide against it; Q436 asks, recommending a priced departure.

**Files.** `crates/pdf-model/src/geospatial/{mod,registration}.rs`, `src/measurement.rs`,
`examples/geospatial_census.rs`; `crates/viewer-core/src/located.rs` (its copy of the fit's tests
moved to `registration.rs`), `tests/measuring.rs`; `tools/drive-windows.sh` (`located`, a fixture
comment); `doc/conformance/ledger.toml` (§12.10.2, §12.10.4); `doc/state-of-play.md`,
`doc/todo/65`; ADR 1918; Q436.

**Gates.** `rustfmt --check`: 0 on six files. `clippy -D warnings --all-targets` `pdf-model`,
`viewer-core`: exit 0. `nextest -p pdf-model -p viewer-core`: 2 429 passed, exit 0. `cargo test -p
conformance`: the ledger checks pass; two failures are siblings' (`viewer-qt` `Host::arrivals`, a
record without gates). Census at HEAD, export `head` (held 195 s, 7.85 GiB): exit 0, 136 s. Census
and the four windows' release builds, export `mine` (held 584 s, 8.13 GiB): exit 0, 127 s. Drive
`located`, `located_displayed` in all four windows: 8 works, 0 wrong, exit 0. `t2-corpus` on
export `mine` (queued 3 700 s, held 38 s): 1 passed, every ratchet at its ceiling, exit 0.
`tools/batch.sh check`: exit 1, on a sibling's record (1542) alone. Duration 8 500 s.
