# 453 — A profile inflated once per mark

**Finding.** Re-surveying all 65 944 web documents named two slow pages, and the slower one spends
**95% of its 380 G interpretation instructions in `ColourSpace::parse_at`** — inflating and reading
one `ICCBased` profile 1053 times, once per `cs` on a page of 1053 distinct axial shadings.

**Date.** 2026-08-12.
**ADR.** [0288](../adr/0288-a-profile-inflated-once-per-mark.md).
**Touched.** `crates/pdf-model/src/content.rs`, `crates/pdf-model/src/shading.rs`,
`doc/todo/03-more-corpora.md`, `doc/todo/41-decoded-stream-cache.md`, `doc/adr/0288-*`, this file.

## The survey

145 archives, one process each, 65 944 documents, 1188 s, 0 failures: 173 unopenable, 45 locked,
23 encrypted beyond us, 52 pageless, **823 incomplete**, 2 slow. 1144 → 823 since ADR 0269.

## What was taken

`Interpreter::icc_spaces` remembers a `[/ICCBased <stream>]` space by the `ObjectId` its resource
entry states — the one space whose meaning cannot depend on the resource dictionary in force.
`3129278.pdf`: **34 450 ms → about 1 550 ms**. `shading::Cache` gained the same table, worth 4% of
that page rather than 95%.

## What was named and not taken

`3990833.pdf` — 279 commands, 24 948 → about 19 500 ms — is 38 images converted sample by sample
through a press. Its profile is in `doc/todo/03` beside the survey line, as the population's next
candidate.

## Gates

Every verdict identical; the corpus gate measurably faster, A/B in one sitting.
