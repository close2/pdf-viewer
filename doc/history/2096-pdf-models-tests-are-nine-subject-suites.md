# 2096 — `pdf-model`'s integration tests are nine subject suites and twelve walks

Session 2096, a measure round in `pdf-viewer-gwg`: the owner's A907, option 2.

- **The layout.** `crates/pdf-model/Cargo.toml` states `autotests = false` and 21 `[[test]]` targets.
  Twelve are the walks the gates name by `--test` (`actions`, `corpus`, `dates`, `drawn_while_read`,
  `fixed_documents`, `jpeg2000`, `oracle`, `raster_golden`, `save_round_trip`, `script_corpus`,
  `text_extraction`, `xmp`), unchanged. Nine are subject suites (`syntax`, `content`, `colour`,
  `transparency`, `images`, `text`, `annotations`, `forms`, `navigation`, each `<subject>_suite`), a
  root file in `tests/` of `mod` lines naming the other 338 files, each placed by the first clause its
  header cites. **No file moved**, so the nineteen owner audit files that `include_bytes!` a fixture by
  a relative path, the `#[path]` helpers and every ledger `test` site stay as they were.
- **The guard.** With discovery off, an unnamed file is compiled by nothing and every command exits 0;
  `tools/conformance/tests/test_targets.rs` fails on one (a planted orphan failed it), and
  `commands.rs` reads a discovery-off crate's targets from `[[test]]` alone.
- **References followed**: `doc/gwg-output-suite.md`, `doc/todo/03`, `doc/traps/pixels-and-rasterisers.md`
  trap 1, and the `--ignored` commands in `restrictions.rs` and `transparency_groups.rs`. No tool, CI
  line or ledger row named a grouped file as a binary.
- **Test count: 2 896 listed before and after**, the sorted lists identical; nextest's summary
  `2874 tests run: 2874 passed (2 slow), 22 skipped` on both trees.
- **ADR 3086** holds the numbers (two sittings, ADR 3036's method, `RUSTC_WRAPPER=""`): the directory
  11 646 → 3 614 MiB, a clean build's processor time 3 009 → 2 524 s by median, a one-file rebuild the
  same wall with incremental off (16.0 → 11.2 s) and 10.7 → 4.6 s with it on.

**Gates.** Over `-p pdf-model -p conformance`: fmt `--check` exit 0; clippy `-D warnings --all-targets`
exit 0; `pdf-sandbox --bins` exit 0; nextest of `pdf-model` (`--lib --bins --tests`'s set) under
`xvfb-run -a` exit 0, 2874 run, 2874 passed, 22 skipped, so `audit-diff.py`'s difference is empty
(failed 0, expected 0; the manifest is unchanged); `conformance` by `cargo test -p conformance
--no-fail-fast` 1 failed, this paragraph's absence, then 0; rustdoc `-D warnings
--document-private-items` exit 0. Fuzz not owed: no public type changed.

Trap: a crate with `autotests = false` compiles nothing it is not told to, and says nothing about it.
