# ADR 0098 — Auditing the population where a false claim can hide

Status: accepted, 2026-08-01.

## Context

`FILE_ONLY_EVIDENCE_CEILING` counts the `implemented` rows whose evidence is a whole test *file*
rather than a named test, and `doc/HANDOVER.md` has called it "the ledger's own next piece of
work" for several sessions. The argument is exact:

> A row naming `file.rs::a_test` is a claim something would fail if it stopped being true. A row
> naming `file.rs` is a claim nothing checks.

Three rows have been found wrong so far — §8.7.3.1's `/BBox` clipping a cell, §8.7.2's pattern
space inside a form, §14.6.2's "both forms" — and **all three had that shape**. Each was found by
the oracle, on a page, after shipping.

## Decision

**Audit a family rather than rename one.** §7.10's ten rows all named `tests/shadings.rs`, which
is a reasonable file for them to be exercised by and no kind of check on what they claim.

`function.rs` already had unit tests for three of the four function types — the exponential
interpolation, the nested stitching, the PostScript operators — and those rows only had to *cite*
them. The type 0 sampled half had none, and writing one is where the audit earned itself.

## Two things wrong in one row

**"All five of Table 39's sample widths are read."** Table 39 says "[v]alid values shall be 1 , 2
, 4 , 8 , 12 , 16 , 24 , and 32", which is **eight**. The code reads all eight and always has;
the row had miscounted the standard. `every_width_table_39_lists_is_read_and_no_other` now encodes
two samples at each width — all zero bits and all one bits — and reads both ends back, which is
where a width read wrongly shows: §7.10.2's map is `Interpolate(sample, 0, 2^BitsPerSample − 1,
…)`, so a wrong width moves the *divisor* rather than the samples. A width the table does not list
is refused rather than rounded, and that is checked too.

**Table 39's `/Order` is read nowhere at all.** "Valid values shall be 1 and 3, specifying linear
and cubic spline interpolation, respectively. Default value: 1." This tree interpolates
multilinearly whatever the entry says — **a silent departure inside a row that said
`implemented`**, which is precisely the thing the ceiling exists to count.

It stays a departure rather than becoming a refusal, and the reason is the clause's: it names a
*cubic spline* and states no spline. Choosing one — Catmull-Rom, natural, something else — would
be inventing an algorithm the standard does not give, and refusing the function outright would
lose a valid document over an entry whose effect is a smoother ramp.

Measured before deciding, which is what stopped this becoming a report nothing fires: **956 type-0
functions across 26 corpus documents, 955 at eight bits and one at sixteen, and not one states
`/Order 3`.** So the departure is invisible in every file we have. It is recorded in the row and
the row is `partial`.

## Consequences

`FILE_ONLY_EVIDENCE_CEILING` falls **58 → 49** — eight rows cite tests instead of files, and one
left the `implemented` population altogether. `implemented` 359 → 358, `partial` 233 → 234.

The lesson is the one the ceiling was built on, with a new half. Three false claims in this
population were found *by the oracle*, on a page, after shipping. This one was found by writing
the test the row should always have named, and it was a claim no page could have caught: no corpus
document exercises `/Order 3`, so nothing would ever have gone wrong visibly. **A row that names a
file can hide a claim the gates cannot reach at all**, and the only instrument for those is
reading the row against the clause and then against the code.

Forty-nine rows are left in the population. The number may only fall.
