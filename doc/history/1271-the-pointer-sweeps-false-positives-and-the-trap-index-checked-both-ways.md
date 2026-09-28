# 1271 — The pointer sweep's false positives, and the trap index checked both ways

Instruments slot, batch forty-one. No ledger status moved. ADR 1379.

**Premise.** `doc/traps/README.md` never said "8 traps carry ~80%"; that figure is in record 1122, which is not
edited. The index's own counted sentences ("Three have moved", "The five group files") now carry no
number, and a paragraph names `tools/state.sh traps` for the shape. Rows 44, 45, 57 said `loop` for
`interactive loop`; trap 55's ADR list gained 0358 from its group file. Rows 53–59 each have their
entry, and `tests/traps.rs` now holds that for every row in both directions (planted: fails).

**`--bin pointers` (ADR 1379).** A `<…>` template is a form; `doc/questions/A<n>` with `Q<n>` present is its own rung, answers
printed by name; ignored, generated and owner-only paths are not carried; hidden files are walked.
Absent 349 (318 standing, 31 corrections) → 247; in the four navigational documents 4 → 0.

**`doc/verify.md`.** Added: `zoom_frame`, `frame_budget`, `ink_ladder`, `stroke_set`, the gesture test, `spot_depth`,
`image_decode_census`, the state sections `navigation`, `departures`, `cited`, `traps`, `remedies`,
`instruments`, `--bin unread`/`pointers`, and `tools/batch.sh commit`. `unread` is not a section; it
runs inside `navigation`. The title's "every instrument" was false: `tools/state.sh instruments`
(new, in `quick`) lists the examples the catalogue does not name (140, then 135 after these lines).

**The four documents.** 11 sentences: state-of-play (a rank-one matrix's fill and stroke, ADRs 1348/1360; push buttons,
ADR 1357; the padded `stream` keyword, ADR 1365; "eleven actions" made countless), crate-map
(`stroke_image`, `form::pressed`, raster's stroke set, the padding, render-cpu's stroke, the action
list), HANDOVER and PLAN (the traps gate). Read by grep against the twelve records, not line by line.

**`doc/todo/65` and the dependency rows.** 17 `partial` + 4 `reported`, each in one place; bucket 1 says it is empty, bucket 5's nil-gain
sentence and its correction became one sentence, the preamble's "three" is gone. `cargo search
bp512`: only `bp512-nestler` 0.2.1 (refused in the note); PR 1914 open, updated 2026-09-18;
`ed448-goldilocks` still 0.14.0-pre.15 — the note's 2026-09-28 date stands. §7.4.9's thirteen
match `DIFFERS_FROM_THE_REFERENCE_SOFTWARE`'s thirteen.

## Gates

clippy `-D warnings` `conformance` 0; rustfmt on my three files 0; `bash -n` 0; nextest
`conformance` 337/338 — the one failure is §12.5.6.23's two test names, 1267's in flight.
