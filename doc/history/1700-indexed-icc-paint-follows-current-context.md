# 1700 — Indexed ICC paint follows current context

Session 1700, 2026-10-22. ADR 2234/2235; §§8.6.6.3 and 11.7.5.3.

**Change.** Normalize an immediate Indexed-to-ICC selection through the existing
lookup API into the retained profile and decoded components. Reuse the direct-ICC
source constructor and complete-context pre-alpha cache. Preserve declared space,
actual operand recovery, initial index zero, pattern priority and overprint policy.
No new cache, source variant, generic family expansion or output-intent change.

**Gates.** Independent fixed-baseline tests: original 13 passed; new Indexed
controls 1 passed, 9 failed intended assertions. Candidate 23 passed, exit 0;
0.23 s tests, 16 s hold, 17 s elapsed, peak 0.28 GiB. Initial scoped clippy found
only similar fixture parameter names; root supplied a lexical rename without
changing assertions. Final warnings-denied all-target clippy/fmt pass, exit 0.
All five isolated omission controls fail intended assertions; driver exit 0,
259 s hold/323 s elapsed, peak 0.30 GiB. First full suite: 2291 passed, 20 failed
because the shared-target mutant replaced a running test artifact; failure preserved.
Serialized full suite: 2311 passed, 21 skipped, exit 0; 66.391 s tests, 169 s hold,
peak 3.66 GiB. Future mutation targets must be private (ADR 2235).

**Cost.** Fixed-baseline/candidate immutable exports retain identical instrumentation;
production delta only colour.rs. Indexed hits, selection and restoration remain
separate from required-current-state work and ordinary ICC/RGB/generic tint controls.
Release builds pass: 89/171 s hold, peaks .84/.89 GiB. Twelve balanced pairs and
48 guards pass, exit 0: 346 s hold, 744 s elapsed, .06 GiB. Indexed repeat/select
paired ratios 1.00856/1.11140; entry/profile/table restore 1.20177/1.06981/1.06027,
adding 1.514/2.186/1.887 ms per20k marks. Required intent/BPC add2.260/2.561 ms;
all other representative pixels agree. Full costs/noisy ranges retained in ADR2235.

**Limits.** §11.7.5.3 remains partial. Other base families and Indexed process-tint
handling remain separate obligations. Root owns combined corpus and integration.
Implementation-to-final-report duration is recorded externally; scoped holds above
are not the round's duration. Combined rendering/launch acceptance remains pending.
