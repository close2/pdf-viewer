# Specification audit — round 2038

Baseline: `b901326c230faa5d2cb288016339b9d5c6288527`, pulled by the owner on
2026-11-16. Branch: `audit/specification-evidence-2026-11-16`. This is a new
audit round carrying forward the [previous inventory](1983-specification-audit.md),
not a restart and not a certification of the updated implementation.

The current ledger has 883 rows: 780 implemented, 10 departed, 8 partial,
2 writer-side, 32 inapplicable and 51 out-of-scope. Prior partial reviews are
retained as provenance; they are not silently promoted to current verification.
The [evidence directory](2038-specification-audit/) records the baseline, per-clause
continuation inventory, and work still pending.

## Method and current work

Recheck all 72 previously failing assertions against the updated code and the
effective ISO specification, including EC3. These are assertion counts, not
distinct-defect counts. Review actual assertion bodies, valid fixture controls
and public consumers independently of execution. Then expand into previously
pending features and selected ADR decisions. The first new shard covers logical
structure and attributes, including effective clause numbering.

All **72 prior failing assertions now pass** at this baseline, including the three
GTK cases run under Xvfb. The broad 11-crate run selected 6,518 tests: 6,508 passed
and ten failed while opening one missing corpus witness. After linking the existing
corpus into this worktree, those exact ten passed. The separate GTK library run
passed all 20 tests. There were 55 skipped tests in the broad run. Execution and
source/assertion reviews remain separate in the evidence files; none of these
counts certifies complete feature coverage.

The 72 scoped reviews revisit parts of 51 ledger clauses. Parent review corrected
historical-log confusion and obsolete ADR residuals before recording the current
assessment. In particular, Qt choices now retain an uncommitted selection; archive
and imported-FDF subset naming have successor implementations (ADRs 2838 and 2862).
Their old residual notes are not evidence of current defects.

New assertion work has already found a remaining speech gap: `/Alt` and `/E`
Unicode language escapes leave the spoken language at the prevailing `/Lang`.
Both controlled tests fail with `en-US` instead of `es`. A three-run `/ActualText`
control passes, including multibyte offsets; ADR 2812's mid-string residual is
obsolete for that path. These are two failing assertions on related handling,
not a count of independent defects.

Date regressions now classify their known valid/invalid fixtures independently
of the parser also used by the writer. FDF callback tests explicitly compare
execution positions, with the `After`/`AfterPermsReady` ordering identified as
ADR 2816's choice rather than a separately stated ISO order. These strengthened
assertions pass. The next batch found six failing assertions: wide/deep structure traversal silently
omits content without its promised truncation flag, and details-view collection
cells omit CI text/date/numeric values and compressed size. The structure
attribute-stream control and PDF-level JBIG2 filter-chain assertions pass.
An encryption control also fails: a `CF/Identity` entry is parsed despite Table20
requiring it to be ignored. There are now nine expected failing assertions; related
failures are grouped rather than counted as independent defects. The combined
28-test run has 19 passes and exactly these nine failures. Artifact reporting and
the two added colour arithmetic/transfer controls pass. A shared-date-parser
mutation is killed by both strengthened invalid-input tests, while valid controls
pass; the source was restored and all date targets then passed.

The next controls expose two additional failures: same-key string destinations prefer
the catalog dictionary over the name tree, and stitching uses signed-zero ordering
where the degenerate endpoint needs numeric equality. The destination finding states
the competing interpretation explicitly. There are eleven expected failing assertions.
Cross-reference fixture repairs and additional ordering/default-width controls pass.
Three public structure-element `ActualText` ligature tests pass; a preliminary
missing-consumer claim was rejected.

Review of ADR0601 supports the existing repeated-letter page labels but revises its
conflict premise: official issue #432 was withdrawn in favor of #593 before the ADR
was written. The static PDF retains stale accepted annotations. Source links and
chronology are recorded in finding D03; the added public AA/BB label control passes.

Further controlled tests bring the current total to fifteen failing assertions:
sampled functions collapse a tiny nonzero domain, a base image accepts a nonnumeric
Decode endpoint without error, and structure-level `ActualText` is duplicated over
two direct content items or omitted when those items belong to descendants. These
last two assertions share one substitution-design finding. An exact-xref paired
font fallback test passes, including independent glyph positions and refusal reporting.
The PLRM3 review also qualifies 32-bit shifts as local policy, while reaffirming
mandatory zero-fill. Deferred Type4 faults still lack host warning integration, a
limit already acknowledged by the ledger and ADR2848.

The owner-authorized redaction-overlay change is recorded in A842 and ADR 2868;
the MaxLen change is recorded in A834 and ADR 2840. Their changed assertions must
be assessed against those current decisions rather than treated as unauthorized
weakening. Review of their behavior and evidence remains scoped and ongoing.

## Budget and continuation

The account's weekly allowance meter read 29% used at the start. The owner asks
to leave 15%; stop new dispatch near 81% used and reserve the interval through
85% for validation, evidence and handoff. Percentages are account quota readings,
not a known fixed token conversion. Preserve unfinished work explicitly rather
than reduce review quality.

No production code, ledger statuses, or historical audit records are changed by
this audit. Tests and findings will be committed and pushed in reviewable batches.
