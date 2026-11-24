# 2212 — Leftovers: the corpus walk, the lock test, and what the output carries

Session 2212, a leftovers round in the `lane3` worktree. ADRs 3549, 3550, 3551. Ledger: no row
moves; no clause's implementation changed.

**The corpus walk.** Corrections, not regressions. `bec8ae273` (audit 2161, ADR 3353) began to say
a text operator outside a text object, drawn as before: `issue9458` (`ET ET`), `bug1065245` (`n ET
Q` with no `BT`), `issue10388_reduced` (`Tm`, `TJ` with no `BT`) joined at train 8596df12, 67 to 70.
`7368382a6` (audit 2175 B05, ADR 3416) says an `/AP` of states with no `/AS` (Table 166):
`issue15557` joined at ff3b7ccd and first stopped the walk with no `whose_defect` row. The row is
added (the file's) and the four names held with their reasons; the walk passes (0).

**The turn path.** Load: the three figures outside at e2008ed8 were judged at 0 outside by the two
trains after it (1dcdab96, b7be816f) on the same bands; no band moved. Those two trains instead
hold eight launch-path instruction counts 1–3 % over their bands, first at 1dcdab96; not this
round's, named in the report.

**The lock test (ADR 3550).** `a_processor_s_adapter_never_waits` judges the order of the section's
end and a hold released only after it; planted (the hook locking a `Cpu` adapter) it fails in 25 s
naming the planted hold.

**PDF/A.** `027613`: its table names 260 objects of a 429-page file; eleven page content streams
the table declares and holds no entry for carry the damaged zlib data, decoded by
`hexadecimal_spans`. Searched in what the output carries (ADR 3549), they are restated; the file is
now refused by name for its content repairs (`string-unterminated`, `integer-past-i64`).
`178360`'s undecodable data is a Type 0 function: refused by name before writing (ADR 3551);
`498264`'s is a `FontFile2`, left to the output's check. A `/DecodeParms` mismatch on a stream
nothing re-encodes stays refused by name, now after the photographs are known. `150277`'s object
629 is damaged too (its zlib check value disagrees): the over-bound values are its garbled bytes,
on page 25, which no row photographs; still refused by name.

**Census.** With the pack against round 2208's: converted counts unchanged in every profile,
`issue19484_1`/`_2` converted; `027613` and `178360` move from an anonymous refusal to named ones; without the pack the same
movements. veraPDF 1.30.3: 5515 written as conforming, 5449 compliant, 46 failing outputs of 14
documents, round 2208's, rule for rule.

**Gates.** Over `-p pdf-model -p render-raster -p pdf-transform -p viewer-qt`: fmt 0; clippy
`-D warnings --all-targets` 0 (after three findings in the new test fixed); `pdf-sandbox --bins`
0; nextest `--lib --bins --tests` 0, 4519 passed, 0 failed against the manifest's 0; rustdoc 0 on
each; corpus walk 0; `cargo test -p conformance` 0. No public type changed, so no fuzz check.
