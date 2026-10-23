# 2235 — Indexed source proof separates entry and context

Status: **accepted** at the merge that took it, `b17ea075` (line amended by ADR 2260). The round
wrote “proposed for integration; scoped correctness and paired cost measured”. Session 1700.

## Decision

Independent controls distinguish three source dependencies: the same decoded entry in different ICC profiles, different entries in the same profile, and different tables that map the same index to different entries. Save/restore forces a previously selected source to paint after another source occupied the bounded memo. Table identity is not itself required when decoded profile and components agree.

Late intent and endpoint black-point controls change context without reselecting a source. Byte 255 reaches the generated profile's black endpoint; byte 128 is 128/255, not one half. Text, stroke, a reused stencil and inherited Forms exercise consumers independently of path fills. Form resources shadow the actual selected name. RGB group expectations preserve eight-bit premultiplied intermediate storage. The Gray-target cold comparison also asserts neutrality and nonwhite paint; it is a supplementary consistency control, not an independent exact Gray oracle or automatic proof of a shared-cache target collision.

Isolated omission tests must fail actual assertions for retention, lookup entry, profile identity, component equality and context. Immediate selections can seed a correct cached colour even when retained metadata is deliberately corrupted; lookup mutation calibration therefore includes a later context change. A compilation error or a test name is not proof that the omitted dependency was exercised. Root-owned oracle files remain immutable through these experiments.

## Cost protocol

Fixed baseline 9cd2c665 and the candidate receive identical test-only instrumentation. Their production delta is only `content/colour.rs`. Existing ordinary ICC, RGB, generic ICC tint and mixed-use populations remain controls. Separate Indexed populations measure repeated hits, selection, entry/profile/table restoration and late intent/BPC. Restoration is a same-output comparison; late context changes require work the stale baseline omitted. Report both without a performance-neutral claim.

Each sample measures twenty whole interpretations of one thousand primary marks, including content parsing and display-list construction. Document opening and representative raster samples are outside timing. The comparison freezes source and actual Cargo artifacts before twelve balanced CPU-2 pairs under an exclusive nice-zero hold, fixed settling and unchanged quiet guards. All samples and failures are retained. Combined integration remains pending.

## Executed proof and artifact isolation

The isolated candidate passes all 23 independent controls. Removing retention, using a wrong lookup entry, omitting profile identity, omitting component equality and omitting complete-context equality each fail their intended assertion with exit 101. The driver exits 0 after 259 s held/323 s elapsed, peak 0.30 GiB. Oracle files retain the handoff hashes; the shared fixture's subsequent parameter rename is lexical only. The Gray control remains supplementary, without a claimed target-key mutation receipt.

The first full suite produced 2291 passes and 20 failures because the mutation export and nextest shared an artifact directory: after nextest released Cargo's build lock, the mutation build relinked its test executable at the same pathname. The frozen failed executable contains the isolated export's path, and the failures exhibit the deliberately omitted context. This is a test-harness failure, not a reason to change production or expectations. All failure logs and the executable hash are retained. After mutation work ended, the shared-tree suite passes 2311 tests with 21 skipped.

Mutation exports must use a dedicated target directory through configuration or an explicit flag. The corrected external driver rejects inherited target-directory environment settings and a target resolving to the shared batch directory; it also requires the target to be inside the isolated export. That guard is reviewed preparation, not a claim that the corrected driver produced the already-recorded receipts. Cargo's build lock alone does not protect a test runner that subsequently executes a mutable artifact pathname.

## Paired cost evidence

Both immutable release builds pass (89/171 s held, .84/.89 GiB peak). All twelve balanced pairs, twenty-four sides and forty-eight unchanged guards pass: exit 0, 346 s held, 744 s elapsed, peak .06 GiB; maximum GPU mean1% and load2.9971. The table reports medians of paired ratios and deltas, not ratios/differences of the independent medians. No sample was discarded.

| Population | Before median ms | After median ms | Paired ratio | Delta ms/20k | Ratio range |
|---|---:|---:|---:|---:|---:|
| icc-solid-repeat | 5.285 | 5.299 | 0.97661 | -0.123 | 0.60145–1.67468 |
| icc-solid-intent | 8.672 | 8.603 | 0.99619 | -0.033 | 0.61134–1.30188 |
| icc-solid-bpc | 14.601 | 15.123 | 1.04106 | +0.595 | 0.90014–1.06983 |
| icc-tint-repeat | 36.588 | 36.982 | 1.00367 | +0.135 | 0.98451–1.04160 |
| icc-tint-many | 137.950 | 137.949 | 0.99859 | -0.192 | 0.96507–1.07153 |
| icc-tint-values | 48.278 | 49.775 | 1.02856 | +1.372 | 0.97368–1.05974 |
| icc-tint-intent | 38.115 | 37.822 | 0.99370 | -0.240 | 0.95396–1.03730 |
| icc-tint-bpc | 44.117 | 44.867 | 1.02572 | +1.139 | 0.97487–1.05267 |
| rgb-solid-repeat | 4.971 | 4.897 | 0.99256 | -0.036 | 0.91288–1.06095 |
| rgb-solid-values | 15.495 | 15.626 | 1.00292 | +0.046 | 0.98028–1.03069 |
| icc-solid-values | 8.596 | 8.364 | 0.97681 | -0.198 | 0.96044–1.02126 |
| icc-solid-profiles | 10.798 | 10.624 | 0.98880 | -0.121 | 0.96813–1.05935 |
| icc-mixed-repeat | 52.748 | 53.227 | 1.01872 | +0.985 | 0.98978–1.05463 |
| icc-mixed-context | 53.409 | 54.371 | 1.02178 | +1.151 | 0.97245–1.06658 |
| icc-mixed-profile | 53.059 | 53.547 | 1.01574 | +0.822 | 0.96940–1.05493 |
| indexed-repeat | 5.001 | 5.021 | 1.00856 | +0.042 | 0.96994–1.06533 |
| indexed-select | 8.006 | 8.860 | 1.11140 | +0.887 | 1.03546–1.18053 |
| indexed-intent | 6.190 | 8.465 | 1.36469 | +2.260 | 1.29879–1.41879 |
| indexed-bpc | 12.284 | 14.880 | 1.20847 | +2.561 | 1.09341–1.24169 |
| indexed-entry-restore | 7.544 | 9.072 | 1.20177 | +1.514 | 1.10755–1.24940 |
| indexed-profile-restore | 31.291 | 33.669 | 1.06981 | +2.186 | 1.00376–1.09432 |
| indexed-table-restore | 31.466 | 33.210 | 1.06027 | +1.887 | 1.00763–1.08461 |

The repeated Indexed case adds a paired median .042 ms per20,000 marks; selection adds .887 ms. Same-output entry/profile/table restoration adds1.514/2.186/1.887 ms (about76/109/94 ns per mark), with20.2/7.0/6.0% paired median increases. This is a measurable correctness cost, not performance neutrality. A restored source can require conversion after another source occupies the bounded slot; no broader cache is introduced to conceal that cost.

Late-intent and BPC populations require work the stale baseline omitted. Representative pixels change from[138,135,151] to Saturation[0,167,255] and from[89,89,89] to compensated black. Their added2.260/2.561 ms per20,000 marks is not an equal-output comparison. Every other representative sample agrees across artifacts. Existing generic tint/mixed/control modes remain separate; short ordinary-ICC repeat samples span a wide paired ratio range(.60145–1.67468), which is retained rather than averaged away as proof of neutrality.

These are generated one-channel Indexed-ICC whole-interpretation workloads, not corpus-wide timings or an isolated conversion microbenchmark. Scoped recommendation is to retain the bounded implementation for required current-state correctness, subject to root's combined rendering and launch gates. No further optimization or source expansion is justified by this measurement alone.
