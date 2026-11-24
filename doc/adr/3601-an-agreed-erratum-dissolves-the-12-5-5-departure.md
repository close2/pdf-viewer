# 3601 — An agreed erratum dissolves §12.5.5's departure, and the row is `implemented`

Session 2225. Status: **accepted**. Context: ADR 0030 (the reading), ADR 1622 (the departure
stands), ADR 3581 (an agreed erratum is the standard), pdf-issues #850.

## Context

§12.5.5's row was `departed` on one sentence: as printed, it composites the appearance's group
with the annotation's `/BM`, `/ca` and `/CA` entries, against Table 166's `/ca` and `/CA` rows ("shall not be
used if the annotation has an appearance stream") and §12.5.2's list of ignored keys. This tree
applies `/BM` to a stored appearance and not the two opacities (ADR 0030). The row said the
departure stays until an errata collection prints the correction into `doc/md/`.

## Decision

pdf-issues #850 (filed by the owner) strikes `s` from *values* and `, ca and CA entries`, and
writes *entry*: the group is composited *using the value of the BM entry in the annotation
dictionary*. The PDF TWG agreed on 2026-11-05 and pdf-issues.pdfa.org publishes the change.
Under ADR 3581 the corrected text is the standard as of its publication. The condition the row
was waiting for is the older one ("an errata collection prints it"), and ADR 3581 replaced it.
So the three sentences agree, the code runs all of them, and the row is **`implemented`**.

No code changes: `annotation::blend_mode` and the opacity defaults already did what the
corrected sentence says. The comments that quoted the struck words (`annotation.rs`,
`content/annotations.rs`, `examples/appearance_transparency_census.rs`) now quote the verbatim
unstruck part. They give the corrected words in italics, citing #850.

## Consequences

The ledger has one fewer `departed` row. ADR 0030 and ADR 1622 are records and stay as they are.
`doc/md/` still carries the printed sentence, so `spec-errata check` keeps reporting it there
until a conversion carries the correction.
