# 1842 — Under `as-if-printed` a part 4 file in `DeviceCMYK` takes the shipped press as its assumed press

Slot 3 of batch 125, 2026-10-28, an "archive" round (Sonnet, high, two sessions); ADRs 2518 and 2519
written, Q708 unused. I worked in `scratchpad/r1842/`. No ledger status moves: an archive round on
R05 (a) (A694 item 2); no clause row changes state.

**Premise** (the press can be assumed for every part 4 file in `DeviceCMYK`): did not hold. The first
build converted 19 documents and veraPDF failed 7 of them at part 4 on ISO 19005-4 section 6.2.4.2:
a `DefaultCMYK` makes `DeviceCMYK` marks `ICCBased` ones (ISO 32000-2 §8.6.5.6) and overprint mode 1
is forbidden for those. This tree's own check does not see it (ADR 2518 section 5), so the route
refuses a file whose survey finds such a mark.

**Built.** `Loss::PressAssumed` (`press-assumed`, granted by `as-if-printed` at the row's site, part 4
only); `supplied_cmyk.rs`: `Press`, the assumed profile and its two sentences; the overprint-mode
guard; the report's `press_assumed` field and the History action; the note that `as-if-printed` is
"as if printed and scanned" in the profile, `--help`, `config.rs`, the mitigations and `doc/todo/72`;
`doc/pdfa-remedies.md` rows. R05 (b) is not built: no ICC table writer exists (ADR 2519, deferred).

**Census** (`archive_corpus`, converted counts, `as-if-printed` part 4, versus HEAD's census of the
same commit): with the CJK pack pdf.js 841 to 846, corpora 385 to 391; without it pdf.js 830 to
835, corpora 385 to 391; 2b and `as-shown` unchanged in all four sections; none newly refused.
Without the guard: 841 to 847 and 385 to 398. Nine documents are refused by the overprint sentence.
veraPDF 1.30.3 at part 4 passed 10 of the 11 freed outputs; `0004-637X_751_2_90.pdf` fails section
6.2.10.5 (glyph widths), as its 2b output fails 6.2.11.5. No pixel gate: no arm exported or compared.

**Gates.** Export `press` (HEAD plus my paths): `cargo test -p pdf-transform --test archive` exit 0 (414 passed,
6 ignored, 7 of them `press_assumed`), `cargo fmt --check -p pdf-transform` exit 0, clippy
`--all-targets -D warnings` exit 0, rustdoc `-D warnings` exit 0; `archive_corpus --ignored` exit 0 (7
passed), `cross_check --ignored` exit 0 (1 passed); the census above exit 0 in both arms. `--bin cited`
exit 0, `--bin quotations` exit 0 (3 diverging, none mine). `cargo test -p conformance --no-fail-fast` exit 101:
`evidence` red on round 1843's record (a path outside the tree, a sibling's) and `records` red on this
record's own Gates paragraph before it was written (re-run below).

After the validator's findings (description, History wording, stopped-short walk, guard on the operator's
press as well, docs): `--test archive` exit 0 (416 passed, 6 ignored, 9 of them `press_assumed`), fmt, clippy
`-D warnings` and rustdoc exit 0, the pack census re-run exit 0 with the same part 4 counts (846 and 391),
`--bin quotations` exit 0, `--test records` exit 0.
**Unfinished.** Seeing section 6.2.4.2's rule behind a `DefaultCMYK` in `survey.rs`; R05 (b). First
session about 150 tool uses; this one about 50 minutes and 40 tool uses. **Validation.** An Opus validator failed the first reading on eight findings — the report and the history entry described the assumed press as one the operator supplied and claimed a provenance nobody knows, the overprint guard ignored a walk stopped short and did not cover the operator's press, and four sentences were out of date — and passed the round after it fixed all eight, its plants re-run.
