# 1331 — A row held only by walks owes a fixture

Ledger slot of batch fifty-one. ADR 1497 (1498 unused). No status moved; no question written.

**Premise checked.** No row is held only by `#[ignore]`d walks: the 30 ignored and 2 `src/bin/`
entries all sit beside a test that runs. The real class is the non-ignored corpus witness, which
reads `doc/pdf.js`, `doc/corpora/` or `doc/veraPDF-corpus` and returns early when it is absent.

**Instrument (ADR 1497).** `ledger.rs`'s `TestClassifier` sorts each `test =` entry as `Fixture`,
`CorpusWitness`, `Walk`, `Census`, `File` or `Missing`, reading rustfmt's layout with no parser.
`bin/ledger`, and so `state.sh ledger`, prints a `held by` line per status (fixture / walks or
witnesses only / no test). An `implemented` row with no fixture is `Problem::OnlyWalks`, admitted
up to `ONLY_WALKS_CEILING` by an `==` assertion. A100, as ratified: a requirement executed under a
control is executed. A present corpus is such a control, so statuses stand and a fixture is owed.

**Before / after** (`--bin ledger`): `implemented` 650 / 35 / 0 to 680 / 5 / 0, and `partial`
8 / 3 / 0 to 10 / 1 / 0.

**Fixtures** (each calibrated by a mutation that makes it fail):
- `pdf-syntax/tests/encryption_fixtures.rs`, 9 tests: revisions 2, 3, 4 (AESV2, unencrypted
  metadata) and 6, encrypted by the test's own Algorithms 1, 1.A, 2, 2.B, 3–5 and 8–10, plus
  §7.6.2's exceptions, `/StmF /Identity` and §7.4.10's `Crypt` naming `Identity`. Added to
  §7.4.10, §7.6, §7.6.2, §7.6.3, §7.6.3.1, §7.6.3.3, §7.6.4, §7.6.4.3, §7.6.4.3.1, §7.6.4.3.3,
  §7.6.4.3.4, §7.6.4.4, §7.6.4.4.3–6, §7.6.4.4.10, §7.6.4.4.11 and §7.6.6.
- `pdf-model/tests/composite_font_fixtures.rs`, 6 tests: embedded one-byte and UTF-8-shaped
  `CMap`s, `90ms-RKSJ-H` by name, §9.7.4.3's EXAMPLES 1–3. Added to §9.2, §9.2.1, §9.7, §9.7.1,
  §9.7.4, §9.7.4.1, §9.7.4.3, §9.7.5.3, §9.7.6 and §9.7.6.1.
- `pdf-model/tests/colour_paths.rs`, one test: §8.6.4.2's levels, its EXAMPLE, the reset to 0.0.
- §12.3.1 and §12.3.2 now name existing fixture unit tests in `outline.rs`, `thumbnail.rs` and
  `destination.rs`.

**Left.** §14.8.2.5.2, §14.8.2.6, §14.8.2.6.2, §14.8.6 and §I.1 (implemented), and §7.4.9
(partial). They are in `doc/todo/65`'s eighth shape, by command.

**Gates.** `cargo test -p conformance`: exit 0 (319 lib tests, every integration test). An
earlier run exited 101 on 1330's `fuzz_workspace`, which has since passed. `cargo nextest run`:
`-p pdf-syntax` exit 0, 294/294; `-p pdf-model` exit 0, 1748/1748. `rustfmt --check` on my six
files: exit 0. `bash -n tools/state.sh`: exit 0. `clippy -D warnings` (conformance, pdf-syntax,
pdf-model): exit 101 from neighbours' `tests/banded_decodes.rs` and `src/content/text.rs`; mine
are clean. No tier 2: only tests were added, so no output changed.
