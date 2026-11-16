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

Fresh sandbox workers have built successfully. An 11-crate baseline suite is
running with 6,518 selected tests and 55 skipped; no current pass/fail conclusion
is recorded yet. Old failing logs are historical evidence only.

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
